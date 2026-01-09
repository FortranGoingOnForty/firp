PROGRAM debug_test
  IMPLICIT NONE
  INTEGER :: i, sum

  sum = 0
  DO i = 1, 5
    sum = sum + i
    PRINT *, 'i =', i, 'sum =', sum
  END DO

  PRINT *, 'Final sum:', sum
END PROGRAM debug_test
