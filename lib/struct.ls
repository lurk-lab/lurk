// Support for structs, leveraging memoization for efficiency.
load("util.ls")

// Crude first-draft of struct implementation.
def(alistStruct, function (fields) {
  return function (op) {
    if (op === 'new'.key) {
      return function (vals) {
        const alist = emit(zip(fields, vals));
        field => cdr(assoc(field, alist))
      }
    }
  }
})

// Crude first-draft of struct implementation.
def(struct, function (args) {
  return function (op) {
    if (op === 'new'.key) {
      vals => field => nth(position(field, args), vals)
    }
  }
})
