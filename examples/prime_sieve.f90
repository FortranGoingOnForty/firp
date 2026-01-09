! Sieve of Eratosthenes - Find all primes up to N
! Features: arrays, nested loops, logical operations
program prime_sieve
  implicit none
  integer, parameter :: N = 100
  logical :: is_prime(N)
  integer :: i, j, count

  print *, 'Sieve of Eratosthenes: Primes up to', N
  print *, '======================================='

  ! Initialize all as potentially prime
  do i = 1, N
    is_prime(i) = .true.
  end do
  is_prime(1) = .false.  ! 1 is not prime

  ! Sieve algorithm
  do i = 2, N
    if (is_prime(i)) then
      ! Mark all multiples of i as not prime
      j = i * 2
      do while (j <= N)
        is_prime(j) = .false.
        j = j + i
      end do
    end if
  end do

  ! Count and print primes
  count = 0
  print *, ''
  print *, 'Prime numbers:'
  do i = 2, N
    if (is_prime(i)) then
      print *, '  ', i
      count = count + 1
    end if
  end do

  print *, ''
  print *, 'Total primes found:', count

end program prime_sieve
