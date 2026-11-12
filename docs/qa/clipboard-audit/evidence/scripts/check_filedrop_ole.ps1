Add-Type -AssemblyName System.Windows.Forms
$list = [System.Windows.Forms.Clipboard]::GetFileDropList()
Write-Output ("OLE FileDropList count: " + $list.Count)
if ($list.Count -gt 0) {
    Write-Output ("first: " + $list[0])
}
$data = [System.Windows.Forms.Clipboard]::GetDataObject()
if ($data) {
    $formats = $data.GetFormats() | Sort-Object
    Write-Output ("OLE formats: " + ($formats -join ", "))
} else {
    Write-Output "OLE DataObject: null"
}
