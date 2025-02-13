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
// !(def alist-struct (lambda (fields)
//                (lambda (op)
//                  ;; Both :new and :type are a bit silly and could be normal functions.
//                  ;; However this allows an interface that abstracts the implementation.
//                  (if (eq op :new)
//                      (lambda (vals)
//                      (let ((alist (emit (zip fields vals))))
//                        (lambda (field)
//                          (cdr (assoc field alist)))))))))

// Crude first-draft of struct implementation.

def(struct, function (args) {
  return function (op) {
    if (op === 'new'.key) {
      vals => field => nth(position(field, args), vals)
    }
  }
})

// !(def struct (lambda (args)
//                (lambda (op)
//                  ;; Both :new and :type are a bit silly and could be normal functions.
//                  ;; However this allows an interface that abstracts the implementation.
//                  (if (eq op :new)
//                      (lambda (vals)
//                        (lambda (field)
//                          (nth (position field args) vals)))))))
