! Recursive factorial function
program factorial_test
  implicit none
  integer :: n, result

  n = 5
  result = factorial(n)
  print *, 'Factorial of', n, 'is', result

contains

  recursive function factorial(n) result(f)
    integer, intent(in) :: n
    integer :: f

    if (n <= 1) then
      f = 1
    else
      f = n * factorial(n - 1)
    end if
  end function factorial

end program factorial_test
