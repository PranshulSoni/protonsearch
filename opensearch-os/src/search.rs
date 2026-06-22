use anyhow::{bail, Result};
use ort::{session::{Session, builder::GraphOptimizationLevel}, value::TensorRef};
use serde::Deserialize;
use tokenizers::Tokenizer;

const CATALOG: &[u8] = include_bytes!("../../assets/catalog.bin");
const MODEL: &[u8] = include_bytes!("../../assets/model/model_int8.onnx");
const TOKENIZER: &[u8] = include_bytes!("../../assets/model/tokenizer.json");

#[derive(Clone)]
pub struct CatalogEntry {
    pub control_name: String,
    pub breadcrumb_path: String,
    pub launch_command: String,
    pub source: String,
}

#[derive(Clone)]
pub struct SearchResult {
    pub entry: CatalogEntry,
    pub score: f32,
}

#[derive(Deserialize)]
struct MetaJson {
    control_name: String,
    breadcrumb_path: String,
    launch_command: String,
    source: String,
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    description: String,
}

pub struct SearchEngine {
    vecs: Vec<f32>,
    meta: Vec<CatalogEntry>,
    n: usize,
    dim: usize,
    session: Session,
    tokenizer: Tokenizer,
}

impl SearchEngine {
    pub fn new() -> Result<Self> {
        if CATALOG.len() < 8 {
            bail!("catalog.bin too small");
        }
        let n   = u32::from_le_bytes(CATALOG[0..4].try_into()?) as usize;
        let dim = u32::from_le_bytes(CATALOG[4..8].try_into()?) as usize;

        let mut off = 8usize;
        let mut vecs = Vec::with_capacity(n * dim);
        let mut meta = Vec::with_capacity(n);

        for _ in 0..n {
            let vb = dim * 4;
            for chunk in CATALOG[off..off + vb].chunks_exact(4) {
                vecs.push(f32::from_le_bytes(chunk.try_into()?));
            }
            off += vb;
            let ml = u16::from_le_bytes(CATALOG[off..off + 2].try_into()?) as usize;
            off += 2;
            let m: MetaJson = serde_json::from_slice(&CATALOG[off..off + ml])?;
            off += ml;
            meta.push(CatalogEntry {
                control_name: m.control_name,
                breadcrumb_path: m.breadcrumb_path,
                launch_command: m.launch_command,
                source: m.source,
            });
        }

        let session = Session::builder()
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .with_intra_threads(2)
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .commit_from_memory(MODEL)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let tokenizer = Tokenizer::from_bytes(TOKENIZER)
            .map_err(|e| anyhow::anyhow!("tokenizer: {e}"))?;

        let mut engine = Self { vecs, meta, n, dim, session, tokenizer };
        let _ = engine.search("settings", 1);
        Ok(engine)
    }

    pub fn search(&mut self, query: &str, top_k: usize) -> Vec<SearchResult> {
        let q = query.trim();
        if q.is_empty() { return vec![]; }

        let qvec = match self.embed(q) {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        let mut scores: Vec<(usize, f32)> = (0..self.n)
            .map(|i| {
                let s = self.vecs[i * self.dim..][..self.dim]
                    .iter().zip(&qvec).map(|(a, b)| a * b).sum();
                (i, s)
            })
            .collect();

        scores.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scores.into_iter().take(top_k)
            .filter(|(_, s)| *s > 0.25)
            .map(|(i, score)| SearchResult { entry: self.meta[i].clone(), score })
            .collect()
    }

    fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        let enc = self.tokenizer.encode(text, true)
            .map_err(|e| anyhow::anyhow!("encode: {e}"))?;

        let ids: Vec<i64>   = enc.get_ids().iter().map(|&x| x as i64).collect();
        let mask: Vec<i64>  = enc.get_attention_mask().iter().map(|&x| x as i64).collect();
        let types: Vec<i64> = enc.get_type_ids().iter().map(|&x| x as i64).collect();
        let seq = ids.len();

        let input_ids_t = TensorRef::<i64>::from_array_view(([1usize, seq], ids.as_slice()))
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let attn_mask_t = TensorRef::<i64>::from_array_view(([1usize, seq], mask.as_slice()))
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let type_ids_t  = TensorRef::<i64>::from_array_view(([1usize, seq], types.as_slice()))
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let outputs = self.session.run(ort::inputs![
            "input_ids"      => input_ids_t,
            "attention_mask" => attn_mask_t,
            "token_type_ids" => type_ids_t,
        ])?;

        let (_, hidden) = outputs["last_hidden_state"].try_extract_tensor::<f32>()?;

        Ok(mean_pool_norm(hidden, &mask, seq, self.dim))
    }
}

fn mean_pool_norm(hidden: &[f32], mask: &[i64], seq: usize, dim: usize) -> Vec<f32> {
    let mut sum = vec![0.0f32; dim];
    let mut count = 0u32;
    for t in 0..seq {
        if mask[t] == 1 {
            for d in 0..dim { sum[d] += hidden[t * dim + d]; }
            count += 1;
        }
    }
    if count > 0 { for x in &mut sum { *x /= count as f32; } }
    let norm = sum.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-8 { for x in &mut sum { *x /= norm; } }
    sum
}
