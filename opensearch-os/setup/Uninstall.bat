@echo off
echo ===================================================
echo             OpenSearch OS Standalone Uninstaller
echo ===================================================
echo.
echo This script will forcefully terminate OpenSearch OS and delete all associated files and data.
echo.
pause
echo.
echo Terminating OpenSearch OS processes...
taskkill /F /IM opensearch-os.exe 2>nul
taskkill /F /IM hermes.exe 2>nul
echo.
echo Deleting application files...
rmdir /S /Q "%LOCALAPPDATA%\Programs\OpenSearch OS" 2>nul
echo.
echo Deleting application data & database...
rmdir /S /Q "%APPDATA%\opensearch-os" 2>nul
echo.
echo Deleting shortcuts...
del "%USERPROFILE%\Desktop\OpenSearch OS.lnk" 2>nul
del "%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\OpenSearch OS.lnk" 2>nul
del "%APPDATA%\Microsoft\Windows\Start Menu\Programs\OpenSearch OS\OpenSearch OS.lnk" 2>nul
del "%APPDATA%\Microsoft\Windows\Start Menu\Programs\OpenSearch OS\Uninstall OpenSearch OS.lnk" 2>nul
rmdir "%APPDATA%\Microsoft\Windows\Start Menu\Programs\OpenSearch OS" 2>nul
echo.
echo Uninstallation completed successfully!
echo.
pause
