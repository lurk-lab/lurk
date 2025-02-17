load("macro.ls") // this will also load util.lurk, so we omit that below. An eventual better system would only load
                 // dependencies once.
// !(load "util.lurk")

def(macroEnv, [])

def(macroEnv, bindMacro(and.q, macroEnv,
                        function (whole) {
                          if (cdr(whole)) {
                            if (cdr(cdr(whole))) {
                              list('if'.q, cadr(whole),
                                   cons(and.q, cddr(whole)))
                              } else {
                                cadr(whole)
                              }
                          } else {
                            t
                          }}))

def(macroEnv, bindMacro(or.q, macroEnv,
                        function (whole) {
                          if(cdr(whole)) {
                            if(cddr(whole)) {
                              list('if'.q, cadr(whole),
                                   cadr(whole), // this is memoized
                                   cons(or.q, cddr(whole)))
                              } else {
                                cadr(whole)
                              }
                          } else {
                            nil
                          }}))

def(macroEnv, bindMacro(cond.q, macroEnv,
                        function(whole) {
                          function aux(clauses) {
                            if (clauses) {
                              if (cdddr(car(clauses))) {
                                error(cons("malformed cond clause", cdddr(car(clauses))))
                                } else {
                                  list('if'.q,
                                       caar(clauses),
                                       cadar(clauses),
                                       aux(cdr(clauses)))
                                  }
                              }
                          }
                          aux(cdr(whole))
                          }))

def(macroEnv, bindMacro(typecase.q, macroEnv,
                        function (whole) {
                          const v = cadr(whole);
                          function aux(clauses) {
                            if (clauses) {
                              if (cddar(clauses)) {
                                error(cons("malformed typecase clause"), cddar(clauses))
                              } else {
                                cons(list(list(typeEqq.q, caar(clauses), v), cadar(clauses)),
                                     aux(cdr(clauses)))
                              }
                            }
                          }
                          cons(cond.q, aux(cddr(whole)))
                          }))

def(macroEnv, bindMacro('case'.q, macroEnv,
                        function (whole) {
                          const v = cadr(whole);
                          function aux (clauses) {
                            if (clauses) {
                              if (cddar(clauses)) {
                                error(cons("malformed case clause", cddar(clauses)))
                              } else {
                                cons(list(list(eqq.q, caar(clauses), v), cadar(clauses)),
                                     aux(cdr(clauses)))
                              }
                            }
                          }
                          cons(cond.q, aux(cddr(whole)))
                        }))
