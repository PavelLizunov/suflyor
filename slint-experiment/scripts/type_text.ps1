# Type text into the focused control via SendKeys (verification only).
param([string]$Text = "")
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.SendKeys]::SendWait($Text)
Write-Output "typed: $Text"
