@echo off
echo ========================================
echo Securanido Parking - Headless Test Runner
echo State-of-the-Art Rust Testing 2026
echo ========================================
echo.

REM Navigate to project root
cd /d "%~dp0.."

REM Run cargo tests
echo Running headless unit and integration tests...
echo.
cargo test --no-fail-fast

if errorlevel 1 (
    echo.
    echo ========================================
    echo SOME TESTS FAILED
    echo ========================================
    pause
    exit /b 1
)

echo.
echo ========================================
echo ALL TESTS PASSED
echo ========================================
pause
