const fs=require("fs");
const p="apps/desktop/src/island/island.css";
const lines=fs.readFileSync(p,"utf8").split(/\r?\n/);
const stack=[]; let inComment=false; let inString=null;
for(let i=0;i<lines.length;i++){
  const line=lines[i]; let s="";
  for(let j=0;j<line.length;j++){
    const c=line[j], n=line[j+1];
    if(inComment){ if(c==="*"&&n==="/"){inComment=false;j++;} continue; }
    if(inString){ if(c===inString && line[j-1]!=="\\") inString=null; continue; }
    if(c==="/"&&n==="*"){inComment=true;j++;continue;}
    if(c==='"'||c==="'"){inString=c;continue;}
    if(c==="{"){stack.push(i+1);}
    else if(c==="}"){ if(stack.length===0) console.log("extra close line",i+1); else stack.pop(); }
  }
}
console.log("unmatched opens:",stack.join(","));
if(stack.length) { const ln=stack[stack.length-1]; for(let k=Math.max(0,ln-6);k<Math.min(lines.length,ln+6);k++) console.log((k+1)+": "+lines[k]); }
