Add-Type -AssemblyName System.Windows.Forms
[Console]::Title = 'SuperDesktop UTIT Window Actions Host'
$form = [System.Windows.Forms.Form]::new()
$form.Text = 'SuperDesktop UTIT Window Actions'
$form.Width = 720
$form.Height = 480
$form.StartPosition = 'CenterScreen'
$form.ShowInTaskbar = $true
[void]$form.ShowDialog()