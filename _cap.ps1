$ErrorActionPreference = "Stop"
$exe = "E:\Users\gxh\Documents\GitHub\slint-bitmap\target\debug\slint-bitmap-demo.exe"
$out = "E:\Users\gxh\Documents\GitHub\slint-bitmap\docs\_gallery_check.png"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class Native {
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int maxCount);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lp);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
[void][Native]::SetProcessDpiAwarenessContext([IntPtr](-4))
$p = Start-Process -FilePath $exe -PassThru
try {
    $hwnd = [IntPtr]::Zero
    for ($i = 0; $i -lt 200; $i++) {
        Start-Sleep -Milliseconds 100
        $p.Refresh()
        [Native]::EnumWindows({ param($h, $l)
            [uint32]$procId = 0
            [void][Native]::GetWindowThreadProcessId($h, [ref]$procId)
            if ($procId -eq $p.Id -and [Native]::IsWindowVisible($h)) {
                $sb = New-Object System.Text.StringBuilder 256
                [void][Native]::GetWindowText($h, $sb, 256)
                if ($sb.ToString() -like "*组件库*") { $script:hwnd = $h; return $false }
            }
            return $true
        }, [IntPtr]::Zero) | Out-Null
        if ($hwnd -ne [IntPtr]::Zero) {
            $r = New-Object Native+RECT
            if ([Native]::GetClientRect($hwnd, [ref]$r)) { if ($r.Right - $r.Left -gt 0) { break } }
        }
    }
    if ($hwnd -eq [IntPtr]::Zero) { throw "gallery not found" }
    Start-Sleep -Milliseconds 1500
    $rect = New-Object Native+RECT
    [void][Native]::GetClientRect($hwnd, [ref]$rect)
    $w = $rect.Right - $rect.Left; $h = $rect.Bottom - $rect.Top
    Add-Type -AssemblyName System.Drawing
    $bmp = New-Object System.Drawing.Bitmap($w, $h)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $hdc = $g.GetHdc()
    [void][Native]::PrintWindow($hwnd, $hdc, 3)
    $g.ReleaseHdc($hdc); $g.Dispose()
    $bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    "captured ${w}x${h}"
} finally {
    if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
}