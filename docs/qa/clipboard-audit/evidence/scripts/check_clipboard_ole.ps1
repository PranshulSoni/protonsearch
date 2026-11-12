Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$image = [System.Windows.Forms.Clipboard]::GetImage()
if ($image) {
    Write-Output ("OLE GetImage: " + $image.Width + "x" + $image.Height)
} else {
    Write-Output "OLE GetImage: null"
}
$data = [System.Windows.Forms.Clipboard]::GetDataObject()
if ($data) {
    $formats = $data.GetFormats() | Sort-Object
    Write-Output ("OLE formats: " + ($formats -join ", "))
    Write-Output ("OLE has CF_DIB via DataObject: " + $data.GetDataPresent("DeviceIndependentBitmap"))
} else {
    Write-Output "OLE DataObject: null"
}
