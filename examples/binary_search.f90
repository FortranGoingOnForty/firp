! Binary search demonstration
! Features: recursion, arrays, divide and conquer
program binary_search_demo
  implicit none
  integer :: arr(15)
  integer :: i, search_val, result

  ! Initialize sorted array
  do i = 1, 15
    arr(i) = i * 10
  end do

  print *, 'Binary Search Demo'
  print *, '=================='
  print *, ''
  print *, 'Sorted array: 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150'
  print *, ''

  ! Search for various values
  search_val = 70
  result = binary_search(1, 15, search_val)
  print *, 'Searching for', search_val, '... Found at index:', result

  search_val = 10
  result = binary_search(1, 15, search_val)
  print *, 'Searching for', search_val, '... Found at index:', result

  search_val = 150
  result = binary_search(1, 15, search_val)
  print *, 'Searching for', search_val, '... Found at index:', result

  search_val = 55
  result = binary_search(1, 15, search_val)
  if (result == -1) then
    print *, 'Searching for', search_val, '... Not found'
  else
    print *, 'Searching for', search_val, '... Found at index:', result
  end if

contains

  recursive function binary_search(low, high, val) result(idx)
    integer, intent(in) :: low, high, val
    integer :: idx
    integer :: mid

    if (low > high) then
      idx = -1
    else
      mid = (low + high) / 2
      if (arr(mid) == val) then
        idx = mid
      else
        if (arr(mid) > val) then
          idx = binary_search(low, mid - 1, val)
        else
          idx = binary_search(mid + 1, high, val)
        end if
      end if
    end if
  end function binary_search

end program binary_search_demo
