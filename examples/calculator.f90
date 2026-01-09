! Simple calculator demonstrating expressions and I/O
! Features: arithmetic operators, functions, formatted output
program calculator
  implicit none
  real :: a, b, result

  ! Get two numbers
  print *, 'Simple Calculator'
  print *, '================'

  a = 10.0
  b = 3.0

  print *, 'a =', a
  print *, 'b =', b
  print *, ''

  ! Basic arithmetic
  print *, 'Basic Operations:'
  result = a + b
  print *, '  a + b =', result

  result = a - b
  print *, '  a - b =', result

  result = a * b
  print *, '  a * b =', result

  result = a / b
  print *, '  a / b =', result

  ! Power and modulo
  print *, ''
  print *, 'Advanced Operations:'
  result = a ** 2
  print *, '  a^2   =', result

  result = mod(a, b)
  print *, '  mod(a,b) =', result

  ! Math functions
  print *, ''
  print *, 'Math Functions:'
  result = sqrt(a)
  print *, '  sqrt(a)  =', result

  result = abs(-a)
  print *, '  abs(-a)  =', result

  result = sin(a)
  print *, '  sin(a)   =', result

  result = cos(a)
  print *, '  cos(a)   =', result

  result = exp(1.0)
  print *, '  exp(1)   =', result

  result = log(a)
  print *, '  log(a)   =', result

end program calculator
