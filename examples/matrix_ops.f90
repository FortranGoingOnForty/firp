! Matrix operations demonstration
! Features: 2D arrays, nested loops, array intrinsics
program matrix_ops
  implicit none
  integer, parameter :: N = 3
  integer :: A(N, N), B(N, N), C(N, N)
  integer :: i, j

  print *, 'Matrix Operations Demo'
  print *, '======================'
  print *, ''

  ! Initialize matrices
  do i = 1, N
    do j = 1, N
      A(i, j) = i + j
      B(i, j) = i * j
    end do
  end do

  ! Matrix addition: C = A + B
  do i = 1, N
    do j = 1, N
      C(i, j) = A(i, j) + B(i, j)
    end do
  end do

  ! Print Matrix A
  print *, 'Matrix A (i+j):'
  do i = 1, N
    print *, '  Row', i, ':', A(i, 1), A(i, 2), A(i, 3)
  end do

  ! Print Matrix B
  print *, ''
  print *, 'Matrix B (i*j):'
  do i = 1, N
    print *, '  Row', i, ':', B(i, 1), B(i, 2), B(i, 3)
  end do

  ! Print Matrix C
  print *, ''
  print *, 'Matrix C = A + B:'
  do i = 1, N
    print *, '  Row', i, ':', C(i, 1), C(i, 2), C(i, 3)
  end do

  ! Matrix statistics
  print *, ''
  print *, 'Array Intrinsics:'
  print *, '  Sum of A:', sum(A)
  print *, '  Sum of B:', sum(B)
  print *, '  Sum of C:', sum(C)
  print *, '  Max in C:', maxval(C)
  print *, '  Min in C:', minval(C)

end program matrix_ops
