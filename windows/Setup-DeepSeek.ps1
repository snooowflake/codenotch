# Run locally. No network, logs, environment variables or command-line key.
# Stores only codenotch:deepseek in the current user's Windows Credential Manager.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class CodenotchCredential {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct Credential {
        public UInt32 Flags, Type;
        public string TargetName, Comment;
        public System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
        public UInt32 CredentialBlobSize;
        public IntPtr CredentialBlob;
        public UInt32 Persist, AttributeCount;
        public IntPtr Attributes;
        public string TargetAlias, UserName;
    }
    [DllImport("advapi32.dll", EntryPoint="CredWriteW", CharSet=CharSet.Unicode, SetLastError=true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool Write(ref Credential credential, UInt32 flags);
}
'@
$form = New-Object System.Windows.Forms.Form
$form.Text = 'Codenotch - DeepSeek API'
$form.Size = New-Object System.Drawing.Size(520,210)
$form.StartPosition = 'CenterScreen'
$label = New-Object System.Windows.Forms.Label
$label.Text = 'Collez votre clé API DeepSeek. Elle sera enregistrée dans le coffre Windows.'
$label.Location = New-Object System.Drawing.Point(15,15)
$label.Size = New-Object System.Drawing.Size(480,40)
$inputBox = New-Object System.Windows.Forms.TextBox
$inputBox.Location = New-Object System.Drawing.Point(15,60)
$inputBox.Width = 475
$inputBox.UseSystemPasswordChar = $true
$save = New-Object System.Windows.Forms.Button
$save.Text = 'Enregistrer'
$save.Location = New-Object System.Drawing.Point(360,105)
$save.Width = 130
$save.Add_Click({
    $keyText = $inputBox.Text.Trim()
    if ($keyText -notmatch '^sk-[A-Za-z0-9_-]+$' -or $keyText.Length -gt 512) {
        [System.Windows.Forms.MessageBox]::Show('Format de clé invalide.') | Out-Null
        return
    }
    $bytes = [Text.Encoding]::UTF8.GetBytes($keyText)
    $ptr = [Runtime.InteropServices.Marshal]::AllocHGlobal($bytes.Length)
    try {
        [Runtime.InteropServices.Marshal]::Copy($bytes,0,$ptr,$bytes.Length)
        $credential = New-Object CodenotchCredential+Credential
        $credential.Type = 1
        $credential.TargetName = 'codenotch:deepseek'
        $credential.UserName = 'DeepSeek API'
        $credential.CredentialBlobSize = $bytes.Length
        $credential.CredentialBlob = $ptr
        $credential.Persist = 2
        if (-not [CodenotchCredential]::Write([ref]$credential,0)) {
            throw 'Enregistrement dans le coffre Windows impossible.'
        }
        $inputBox.Clear()
        [System.Windows.Forms.MessageBox]::Show('Clé enregistrée. Relancez Codenotch ou attendez cinq minutes.') | Out-Null
        $form.Close()
    } catch {
        [System.Windows.Forms.MessageBox]::Show('Enregistrement impossible dans le coffre Windows.') | Out-Null
    } finally {
        for ($i=0; $i -lt $bytes.Length; $i++) { [Runtime.InteropServices.Marshal]::WriteByte($ptr,$i,0) }
        [Runtime.InteropServices.Marshal]::FreeHGlobal($ptr)
        [Array]::Clear($bytes,0,$bytes.Length)
        $keyText = $null
    }
})
$form.Controls.AddRange(@($label,$inputBox,$save))
$form.AcceptButton = $save
$form.ShowDialog() | Out-Null
$form.Dispose()
