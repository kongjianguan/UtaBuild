# PowerShell脚本：向UtaBuild窗口发送按键
param(
    [string]$Text,
    [string]$WindowTitle = "UtaBuild"
)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName Microsoft.VisualBasic

# 找窗口句柄
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class Win32 {
    [DllImport("user32.dll", CharSet=CharSet.Auto)]
    public static extern IntPtr FindWindow(string lpClassName, string lpWindowName);
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
}
"@

$hwnd = [Win32]::FindWindow($null, $WindowTitle)
if ($hwnd -eq [IntPtr]::Zero) {
    Write-Output "窗口未找到: $WindowTitle"
    exit 1
}

[Win32]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 200

# Ctrl+A 全选已有内容
[System.Windows.Forms.SendKeys]::SendWait("^a")
Start-Sleep -Milliseconds 100

# 直接发送文本（不经过输入法）
[System.Windows.Forms.SendKeys]::SendWait($Text)

Write-Output "OK: 已发送 '$Text'"
