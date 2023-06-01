#include <Windows.h>
#include <iostream>

// Maximum movement range
int MAX_MOVE_X = 50;
int MAX_MOVE_Y = 50;

boolean isTaskbarAutoHidden()
{
    APPBARDATA appBarData;
    appBarData.cbSize = sizeof(APPBARDATA);
    if (SHAppBarMessage(ABM_GETSTATE, &appBarData) & ABS_AUTOHIDE)
    {
        return TRUE;
    }
    return FALSE;
}

int getTaskbarHeight()
{
    RECT workAreaRect;
    SystemParametersInfo(SPI_GETWORKAREA, 0, &workAreaRect, 0);
    int screenHeight = GetSystemMetrics(SM_CYSCREEN);
    return screenHeight - (workAreaRect.bottom - workAreaRect.top);
}


BOOL CALLBACK EnumWindowsProc(HWND hWnd, LPARAM lParam)
{
    // Check if the window is visible
    if (IsWindowVisible(hWnd))
    {
        // Get the window placement
        WINDOWPLACEMENT wp;
        wp.length = sizeof(WINDOWPLACEMENT);
        GetWindowPlacement(hWnd, &wp);

        // Check if the window is not maximized
        if (wp.showCmd != SW_SHOWMAXIMIZED)
        {
            // Get screen information
            HMONITOR hMonitor = MonitorFromWindow(hWnd, MONITOR_DEFAULTTONEAREST);
            MONITORINFO monitorInfo;
            monitorInfo.cbSize = sizeof(MONITORINFO);
            GetMonitorInfo(hMonitor, &monitorInfo);

            // Calculate screen boundaries
            int screenWidth = monitorInfo.rcMonitor.right - monitorInfo.rcMonitor.left;
            int screenHeight = monitorInfo.rcMonitor.bottom - monitorInfo.rcMonitor.top;

            // Get window size
            int windowWidth = wp.rcNormalPosition.right - wp.rcNormalPosition.left;
            int windowHeight = wp.rcNormalPosition.bottom - wp.rcNormalPosition.top;

            // Calculate maximum movement range
            int maxMoveX = min(MAX_MOVE_X, screenWidth - windowWidth);
            int maxMoveY = min(MAX_MOVE_Y, screenHeight - windowHeight);

            // Calculate random position within the limited range, accounting for window size
            int randomX = wp.rcNormalPosition.left + (rand() % (2 * maxMoveX + 1)) - maxMoveX;
            int randomY = wp.rcNormalPosition.top + (rand() % (2 * maxMoveY + 1)) - maxMoveY;

            // Ensure the new position is within the screen boundaries
            randomX = max(monitorInfo.rcMonitor.left, min(randomX, monitorInfo.rcMonitor.right - windowWidth));
            randomY = max(monitorInfo.rcMonitor.top, min(randomY, monitorInfo.rcMonitor.bottom - windowHeight));

            // Adjust position to account for taskbar
            if (isTaskbarAutoHidden())
            {
                int taskbarHeight = getTaskbarHeight();
                randomY = max(randomY, monitorInfo.rcMonitor.top + taskbarHeight);
            }
            else
            {
                int taskbarHeight = getTaskbarHeight();
                randomY = min(randomY, monitorInfo.rcMonitor.bottom - windowHeight - taskbarHeight);
            }

            std::cout << "Taskbar hidden: " << (isTaskbarAutoHidden() ? "true" : "false") << "\n";
            std::cout << "Taskbar height: " << getTaskbarHeight() << "px \n";

            // Move the window to the random position
            SetWindowPos(hWnd, HWND_TOP, randomX, randomY, 0, 0, SWP_NOSIZE | SWP_NOZORDER);

            // Animate the window movement
            if (!AnimateWindow(hWnd, 4000, AW_CENTER))
            {
                std::cerr << "Failed to animate window movement: " << GetLastError() << std::endl;
            }
        }
    }

    // Continue enumeration
    return TRUE;
}


void CALLBACK TimerProc(HWND hWnd, UINT uMsg, UINT_PTR idEvent, DWORD dwTime)
{
    // Enumerate all windows and move non-maximized ones
    EnumWindows(EnumWindowsProc, 0);
}

int main()
{
    // Seed the random number generator
    srand(static_cast<unsigned int>(time(NULL)));

    // Enumerate all windows
    EnumWindows(EnumWindowsProc, 0);

    // Set a timer with an interval of 2000 ms (2 seconds)
    UINT_PTR timerId = SetTimer(NULL, 0, 2000, TimerProc);

    if (timerId == 0)
    {
        MessageBox(NULL, L"Failed to set timer!", L"Error", MB_ICONERROR | MB_OK);
        return 1;
    }

    // Run a message loop to keep the application running
    MSG msg;
    while (GetMessage(&msg, NULL, 0, 0))
    {
        TranslateMessage(&msg);
        DispatchMessage(&msg);
    }

    // Clean up the timer
    KillTimer(NULL, timerId);

    return 0;
}
