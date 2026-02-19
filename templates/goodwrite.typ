#let procedure(body) = {
  metadata("goodwrite:mode:procedural")
  body
}

#let description(body) = {
  metadata("goodwrite:mode:descriptive")
  body
}

#let warning(body) = {
  metadata("goodwrite:mode:safety")
  block(fill: rgb("#fff3cd"), inset: 8pt, [*WARNING:* #upper(body)])
}

#let caution(body) = {
  metadata("goodwrite:mode:safety")
  block(fill: rgb("#cfe2ff"), inset: 8pt, [*CAUTION:* #upper(body)])
}

#let note(body) = {
  metadata("goodwrite:mode:note")
  block(inset: (left: 16pt), [*NOTE:* #body])
}
