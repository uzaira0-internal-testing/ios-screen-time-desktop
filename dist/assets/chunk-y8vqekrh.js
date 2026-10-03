function C(z,A){let j=z.processing_metadata?.preprocessing;if(!j)return null;let{current_events:q,events:w}=j;if(!q||!w)return null;let x=q[A];if(!x)return null;return w.find((B)=>B.event_id===x)??null}
export{C as h};
