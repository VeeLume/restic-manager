@echo off
echo ========================================
echo Restic Manager Integration Test Cleanup
echo ========================================
echo.

echo [1/4] Stopping Docker container...
docker-compose -f docker-compose.test.yml down -v
echo Done.
echo.

echo [2/4] Removing test data...
if exist "test-data" rmdir /s /q test-data
echo Done.
echo.

echo [3/4] Removing backup repository...
if exist "test-backup-repo" rmdir /s /q test-backup-repo
echo Done.
echo.

echo [4/4] Removing temporary files...
if exist "pre-hook-test.txt" del pre-hook-test.txt
echo Done.
echo.

echo ========================================
echo Cleanup Complete
echo ========================================
