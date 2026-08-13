[CmdletBinding()]
param(
    [Parameter()]
    [string]$WorkspaceRoot = (Split-Path -Parent $PSScriptRoot),

    [Parameter()]
    [string]$BuildDirectory,

    [Parameter()]
    [string]$Fixture
)

$ErrorActionPreference = 'Stop'

function Test-IdentityRecords {
    param([object[]]$Records)

    $required = @('name', 'app_user_model_id', 'original_filename', 'company_name', 'product_name')
    foreach ($record in $Records) {
        foreach ($field in $required) {
            $property = $record.PSObject.Properties[$field]
            if ($null -eq $property -or [string]::IsNullOrWhiteSpace([string]$property.Value)) {
                throw "IDENTITY_MISSING_FIELD: $($record.name) has no $field."
            }
        }
    }

    foreach ($field in @('app_user_model_id', 'original_filename')) {
        $duplicates = $Records | Group-Object -Property $field | Where-Object Count -gt 1
        if ($duplicates) {
            throw "IDENTITY_COLLISION: duplicate $field '$($duplicates[0].Name)'."
        }
    }
}

if ($Fixture) {
    $fixturePath = Join-Path $WorkspaceRoot $Fixture
    if (-not (Test-Path $fixturePath)) {
        throw "Fixture does not exist: $Fixture"
    }
    $fixtureRecords = Get-Content -Raw -Encoding UTF8 $fixturePath | ConvertFrom-Json
    Test-IdentityRecords $fixtureRecords
    throw "Fixture did not violate identity policy: $Fixture"
}

if (-not ("SuperDesktopNativeResource" -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;

public static class SuperDesktopNativeResource {
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr LoadLibrary(string fileName);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool FreeLibrary(IntPtr module);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr FindResource(IntPtr module, IntPtr name, IntPtr type);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr LoadResource(IntPtr module, IntPtr resource);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr LockResource(IntPtr resourceData);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern uint SizeofResource(IntPtr module, IntPtr resource);

    public static byte[] ReadAppUserModelIdResource(string path) {
        IntPtr module = LoadLibrary(path);
        if (module == IntPtr.Zero) throw new InvalidOperationException("LoadLibrary failed.");
        try {
            IntPtr resource = FindResource(module, (IntPtr)101, (IntPtr)10); // RCDATA
            if (resource == IntPtr.Zero) throw new InvalidOperationException("RCDATA identity resource 101 is missing.");
            uint size = SizeofResource(module, resource);
            IntPtr data = LockResource(LoadResource(module, resource));
            if (data == IntPtr.Zero || size == 0) throw new InvalidOperationException("Identity resource cannot be read.");
            byte[] bytes = new byte[size];
            Marshal.Copy(data, bytes, 0, (int)size);
            return bytes;
        } finally {
            FreeLibrary(module);
        }
    }

    public static uint IconGroupSize(string path) {
        IntPtr module = LoadLibrary(path);
        if (module == IntPtr.Zero) throw new InvalidOperationException("LoadLibrary failed.");
        try {
            IntPtr resource = FindResource(module, (IntPtr)201, (IntPtr)14); // RT_GROUP_ICON
            if (resource == IntPtr.Zero) throw new InvalidOperationException("Group icon resource 201 is missing.");
            return SizeofResource(module, resource);
        } finally {
            FreeLibrary(module);
        }
    }
}
'@
}

if ([string]::IsNullOrWhiteSpace($BuildDirectory)) {
    $BuildDirectory = Join-Path $WorkspaceRoot 'target/debug'
}

$records = @(
    [PSCustomObject]@{ name = 'app'; app_user_model_id = 'com.superdesktop.shell'; original_filename = 'SuperDesktop.exe'; company_name = 'SuperDesktop'; product_name = 'SuperDesktop'; internal_name = 'superdesktop-app'; file_description = 'SuperDesktop Shell'; binary = 'superdesktop-app.exe' },
    [PSCustomObject]@{ name = 'guardian'; app_user_model_id = 'com.superdesktop.guardian'; original_filename = 'SuperDesktopGuardian.exe'; company_name = 'SuperDesktop'; product_name = 'SuperDesktop'; internal_name = 'superdesktop-guardian'; file_description = 'SuperDesktop Guardian'; binary = 'superdesktop-guardian.exe' },
    [PSCustomObject]@{ name = 'test-support'; app_user_model_id = 'com.superdesktop.test-support'; original_filename = 'SuperDesktopTestSupport.exe'; company_name = 'SuperDesktop'; product_name = 'SuperDesktop'; internal_name = 'superdesktop-test-support'; file_description = 'SuperDesktop Test Support'; binary = 'superdesktop-test-support.exe' }
)
Test-IdentityRecords $records

$results = foreach ($record in $records) {
    $binaryPath = Join-Path $BuildDirectory $record.binary
    if (-not (Test-Path $binaryPath)) {
        throw "IDENTITY_BINARY_MISSING: $binaryPath"
    }

    $version = [Diagnostics.FileVersionInfo]::GetVersionInfo((Resolve-Path $binaryPath))
    foreach ($field in @('CompanyName', 'ProductName', 'OriginalFilename', 'InternalName', 'FileDescription')) {
        $expectedProperty = switch ($field) {
            'CompanyName' { 'company_name' }
            'ProductName' { 'product_name' }
            'OriginalFilename' { 'original_filename' }
            'InternalName' { 'internal_name' }
            'FileDescription' { 'file_description' }
        }
        if ($version.$field -ne $record.$expectedProperty) {
            throw "IDENTITY_VERSIONINFO_MISMATCH: $($record.name) $field expected '$($record.$expectedProperty)' but got '$($version.$field)'."
        }
    }
    if ($version.FileVersion -ne '0.1.0.0' -or $version.ProductVersion -ne '0.1.0.0') {
        throw "IDENTITY_VERSION_MISMATCH: $($record.name) must report 0.1.0.0."
    }

    $resourceBytes = [SuperDesktopNativeResource]::ReadAppUserModelIdResource((Resolve-Path $binaryPath))
    $actualAppUserModelId = [Text.Encoding]::ASCII.GetString($resourceBytes).Trim([char]0)
    if ($actualAppUserModelId -ne $record.app_user_model_id) {
        throw "IDENTITY_AUMID_MISMATCH: $($record.name) expected '$($record.app_user_model_id)' but got '$actualAppUserModelId'."
    }
    $iconResourceSize = [SuperDesktopNativeResource]::IconGroupSize((Resolve-Path $binaryPath))
    if ($iconResourceSize -eq 0) {
        throw "IDENTITY_ICON_MISSING: $($record.name) icon group resource is empty."
    }

    [PSCustomObject]@{
        name = $record.name
        binary = (Resolve-Path $binaryPath).Path
        app_user_model_id = $actualAppUserModelId
        company_name = $version.CompanyName
        product_name = $version.ProductName
        original_filename = $version.OriginalFilename
        internal_name = $version.InternalName
        file_version = $version.FileVersion
        product_version = $version.ProductVersion
        icon_resource_size = $iconResourceSize
    }
}

$results | ConvertTo-Json -Depth 3
