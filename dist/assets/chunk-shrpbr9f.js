import{Za as q,kb as A}from"./chunk-63vbw7xs.js";function G(){return A(q("hardwareConcurrency"),q("deviceMemory"))}async function J(f,D,E=G()){if(f.length===0)return;let v=0,F=Math.min(Math.max(1,Math.floor(E)),f.length);async function z(){let j=v;if(v+=1,j>=f.length)return;try{await D(f[j],j)}catch{}await z()}await Promise.all(Array.from({length:F},()=>z()))}
export{J as Ya};
