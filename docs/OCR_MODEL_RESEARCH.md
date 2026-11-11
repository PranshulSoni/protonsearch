# OCR model research — Phase 1

Current ProtonSearch uses Windows.Media.Ocr. No external model-pack consumer exists yet. No model files were downloaded during this investigation.

Candidates for an equal-fixture Windows CPU benchmark:

| Engine | Available evidence | Still required |
|---|---|---|
| Windows.Media.Ocr | Existing local backend; 100 synthetic fixture images processed | Multilingual accuracy, larger mixed corpus, performance and memory |
| PP-OCRv6 tiny, ONNX | Official PaddlePaddle detector and recognizer model cards identify Apache-2.0; recognizer file is 4,462,639 bytes | Verify all configuration/dictionary hashes and licenses, integrate detection and recognition, CPU runtime, measure accuracy and resources |
| Indra OCR | Lightweight ONNX models support recognition and layout; model weights have a modified OpenRAIL-M license | Review license before download, verify all required files and hashes, integrate runtime, and benchmark |
| Tesseract `tessdata_fast` | Official documentation describes fast and best language data sets | Verify Windows binary provenance and model hashes; benchmark identical fixtures |

References: [PaddlePaddle recognition model](https://huggingface.co/PaddlePaddle/PP-OCRv6_tiny_rec_onnx), [official ONNX conversion guidance](https://paddlepaddle.github.io/PaddleOCR/main/en/version3.x/deployment/obtaining_onnx_models.html), [Tesseract language-data documentation](https://tesseract-ocr.github.io/tessdoc/Data-Files.html), [ONNX Runtime C deployment documentation](https://onnxruntime.ai/docs/get-started/with-c.html).

Model download and installation must require explicit user consent, pin every required file and checksum, stage and verify before promotion, support cancellation and rollback, and retain the existing offline OCR fallback. Neither weights nor runtime should be represented as usable until the full detection, recognition and dictionary pipeline has been verified. No speed or accuracy comparison has yet been performed on this machine.

Indra's [model license](https://huggingface.co/datalab-to/indra-ocr/blob/main/LICENSE) imposes restrictions beyond an ordinary permissive open-source license. It must be presented for acceptance before downloading those weights. Approximate model sizes do not include the inference runtime or optional language files.
