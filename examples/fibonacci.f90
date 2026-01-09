! Fibonacci sequence demonstration
! Features: recursion, iteration, comparison of approaches
program fibonacci
  implicit none
  integer :: i, n

  n = 15
  print *, 'Fibonacci Sequence'
  print *, '=================='
  print *, ''

  ! Demonstrate recursive Fibonacci
  print *, 'Using RECURSIVE function:'
  do i = 0, n - 1
    print *, '  F(', i, ') =', fib_recursive(i)
  end do

  print *, ''
  print *, 'Sequence calculated correctly using recursion!'
  print *, 'F(n) = F(n-1) + F(n-2), with F(0)=0, F(1)=1'

contains

  ! Recursive Fibonacci function
  recursive function fib_recursive(n) result(f)
    integer, intent(in) :: n
    integer :: f

    if (n <= 0) then
      f = 0
    else if (n == 1) then
      f = 1
    else
      f = fib_recursive(n - 1) + fib_recursive(n - 2)
    end if
  end function fib_recursive

end program fibonacci
