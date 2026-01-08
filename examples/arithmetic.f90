! Arithmetic operations example
program arithmetic
  implicit none
  integer :: x, y, z
  real :: a, b, c

  ! Integer arithmetic
  x = 10
  y = 3
  z = x + y * 2  ! z = 16

  ! Real arithmetic
  a = 3.14
  b = 2.0
  c = a * b ** 2  ! c = 12.56

  print *, 'Integer result:', z
  print *, 'Real result:', c
end program arithmetic
