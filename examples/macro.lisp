(bind* (macro . (lambda (f) (obj 'macro f)))
       (inner-fn . objrest)
       (let . (macro (lambda (binds body) ((closure (apply (inner-fn bind) (cons scope binds)) () body)))))
       (let1 . (macro (lambda (sym val body)
              (list `(lambda `(,,sym) ,body) val))))
       (y . (let1 x 2 x)))