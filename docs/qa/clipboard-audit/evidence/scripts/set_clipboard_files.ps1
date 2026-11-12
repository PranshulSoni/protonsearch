Add-Type -AssemblyName System.Windows.Forms
$path = Join-Path $PSScriptRoot '..\runtime\file-drop-source.txt'
$path = [System.IO.Path]::GetFullPath($path)
Set-Content -Path $path -Value 'clipboard file drop source'
$data = New-Object System.Windows.Forms.DataObject
$data.SetData([System.Windows.Forms.DataFormats]::FileDrop, @($path))
[System.Windows.Forms.Clipboard]::SetDataObject($data, $true)
Write-Output ("file-drop-set: " + $path)
