#let requirement(id, body) = {
  metadata("goodwrite:requirement")
  block(inset: 8pt, stroke: (left: 2pt + rgb("#0066cc")), fill: rgb("#f0f6ff"))[
    #text(weight: "bold", fill: rgb("#0066cc"))[#id] #h(6pt) #body
  ]
}

#let requirement_ubiquitous(id, body) = {
  metadata("goodwrite:requirement:ubiquitous")
  block(inset: 8pt, stroke: (left: 2pt + rgb("#0066cc")), fill: rgb("#f0f6ff"))[
    #text(weight: "bold", fill: rgb("#0066cc"))[#id] #h(6pt) #body
  ]
}

#let requirement_event(id, body) = {
  metadata("goodwrite:requirement:event-driven")
  block(inset: 8pt, stroke: (left: 2pt + rgb("#0066cc")), fill: rgb("#f0f6ff"))[
    #text(weight: "bold", fill: rgb("#0066cc"))[#id] #h(6pt) #body
  ]
}

#let requirement_event_driven(id, body) = {
  metadata("goodwrite:requirement:event-driven")
  block(inset: 8pt, stroke: (left: 2pt + rgb("#0066cc")), fill: rgb("#f0f6ff"))[
    #text(weight: "bold", fill: rgb("#0066cc"))[#id] #h(6pt) #body
  ]
}

#let requirement_state(id, body) = {
  metadata("goodwrite:requirement:state-driven")
  block(inset: 8pt, stroke: (left: 2pt + rgb("#0066cc")), fill: rgb("#f0f6ff"))[
    #text(weight: "bold", fill: rgb("#0066cc"))[#id] #h(6pt) #body
  ]
}

#let requirement_state_driven(id, body) = {
  metadata("goodwrite:requirement:state-driven")
  block(inset: 8pt, stroke: (left: 2pt + rgb("#0066cc")), fill: rgb("#f0f6ff"))[
    #text(weight: "bold", fill: rgb("#0066cc"))[#id] #h(6pt) #body
  ]
}

#let requirement_unwanted(id, body) = {
  metadata("goodwrite:requirement:unwanted")
  block(inset: 8pt, stroke: (left: 2pt + rgb("#cc6600")), fill: rgb("#fff8f0"))[
    #text(weight: "bold", fill: rgb("#cc6600"))[#id !] #h(6pt) #body
  ]
}

#let requirement_optional(id, body) = {
  metadata("goodwrite:requirement:optional")
  block(inset: 8pt, stroke: (left: 2pt + rgb("#0066cc")), fill: rgb("#f0f6ff"))[
    #text(weight: "bold", fill: rgb("#0066cc"))[#id] #h(6pt) #body
  ]
}

#let requirement_complex(id, body) = {
  metadata("goodwrite:requirement:complex")
  block(inset: 8pt, stroke: (left: 2pt + rgb("#0066cc")), fill: rgb("#f0f6ff"))[
    #text(weight: "bold", fill: rgb("#0066cc"))[#id] #h(6pt) #body
  ]
}
