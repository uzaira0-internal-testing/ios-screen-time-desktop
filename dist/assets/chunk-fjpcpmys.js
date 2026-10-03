function f(d){if(d===null||d===void 0)return"";if(typeof d==="string")return d;if(typeof d==="number"||typeof d==="boolean"||typeof d==="bigint")return String(d);if(d instanceof Error)return d.message;try{return JSON.stringify(d)??Object.prototype.toString.call(d)}catch{return Object.prototype.toString.call(d)}}
export{f as da};
