! Sorting algorithms demonstration
! Features: array manipulation, loops, comparisons
program sorting
  implicit none
  integer :: arr(10)
  integer :: i, j, n, temp
  logical :: swapped

  n = 10

  ! Initialize with unsorted data
  arr(1) = 64
  arr(2) = 34
  arr(3) = 25
  arr(4) = 12
  arr(5) = 22
  arr(6) = 11
  arr(7) = 90
  arr(8) = 45
  arr(9) = 77
  arr(10) = 33

  print *, 'Sorting Algorithms Demo'
  print *, '======================='
  print *, ''
  print *, 'Original array:'
  do i = 1, n
    print *, '  a(', i, ') =', arr(i)
  end do

  ! Bubble sort
  do i = 1, n - 1
    swapped = .false.
    do j = 1, n - i
      if (arr(j) > arr(j + 1)) then
        ! Swap elements
        temp = arr(j)
        arr(j) = arr(j + 1)
        arr(j + 1) = temp
        swapped = .true.
      end if
    end do
    ! Early exit if sorted
    if (.not. swapped) exit
  end do

  print *, ''
  print *, 'After Bubble Sort:'
  do i = 1, n
    print *, '  a(', i, ') =', arr(i)
  end do

  print *, ''
  print *, 'Statistics:'
  print *, '  Min value:', minval(arr)
  print *, '  Max value:', maxval(arr)
  print *, '  Sum:', sum(arr)

end program sorting
