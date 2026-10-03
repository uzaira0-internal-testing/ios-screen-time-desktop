class r extends Error{existingId;constructor(e){super(`Duplicate image: already uploaded as screenshot #${e}`);this.name="DuplicateScreenshotError",this.existingId=e}}
export{r as ca};
