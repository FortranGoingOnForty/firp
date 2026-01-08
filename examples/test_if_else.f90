program test
  integer :: x
  if (x > 0) then
    print *, 'Positive'
  else if (x == 0) then
    print *, 'Zero'
  else
    print *, 'Negative'
  end if
end program test
