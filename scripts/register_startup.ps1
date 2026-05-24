# 1. 设置基础信息 (请根据你的实际情况修改)
$targetExe = "\path\to\ChronoSnap.exe"     # 你的 Release 版 exe 绝对路径
$shortcutPath = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\ChronoSnap.lnk" # 快捷方式存放点
$appId = "top.volan.chrono-snap"            # 你的 AppUserModelID

# 2. 定义修正后的 C# 助手类
$code = @"
using System;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text;

public class ShellLinkHelper {
    // 正确的 IShellLinkW 接口 ID
    [ComImport, Guid("000214F9-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IShellLinkW {
        void GetPath([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszFile, int cchMaxPath, out IntPtr pfd, uint fFlags);
        void GetIDList(out IntPtr ppidl);
        void SetIDList(IntPtr pidl);
        void GetDescription([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszName, int cchMaxName);
        void SetDescription([MarshalAs(UnmanagedType.LPWStr)] string pszName);
        void GetWorkingDirectory([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszDir, int cchMaxPath);
        void SetWorkingDirectory([MarshalAs(UnmanagedType.LPWStr)] string pszDir);
        void GetArguments([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszArgs, int cchMaxPath);
        void SetArguments([MarshalAs(UnmanagedType.LPWStr)] string pszArgs);
        void GetHotkey(out ushort pwHotkey);
        void SetHotkey(ushort wHotkey);
        void GetShowCmd(out int piShowCmd);
        void SetShowCmd(int iShowCmd);
        void GetIconLocation([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszIconPath, int cchIconPath, out int piIcon);
        void SetIconLocation([MarshalAs(UnmanagedType.LPWStr)] string pszIconPath, int iIcon);
        void SetRelativePath([MarshalAs(UnmanagedType.LPWStr)] string pszPathRel, uint dwReserved);
        void Resolve(IntPtr hwnd, uint fFlags);
        void SetPath([MarshalAs(UnmanagedType.LPWStr)] string pszFile);
    }

    [ComImport, Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IPropertyStore {
        int GetCount(out uint cProps);
        int GetAt(uint iProp, out IntPtr pkey);
        int GetValue(IntPtr pkey, [In, Out] PropVariant pv);
        int SetValue(ref PropertyKey pkey, [In] PropVariant pv);
        int Commit();
    }

    [StructLayout(LayoutKind.Sequential, Pack = 4)]
    struct PropertyKey {
        public Guid fmtid;
        public uint pid;
    }

    [StructLayout(LayoutKind.Explicit)]
    public class PropVariant : IDisposable {
        [FieldOffset(0)] public ushort vt;
        [FieldOffset(8)] public IntPtr ptr;
        public PropVariant(string value) {
            vt = 31; // VT_LPWSTR
            ptr = Marshal.StringToCoTaskMemUni(value);
        }
        public void Dispose() {
            if (ptr != IntPtr.Zero) { Marshal.FreeCoTaskMem(ptr); ptr = IntPtr.Zero; }
        }
    }

    public static void CreateShortcutWithId(string linkPath, string targetExe, string appId) {
        // ShellLink 的类 ID
        Type shellLinkType = Type.GetTypeFromCLSID(new Guid("00021401-0000-0000-C000-000000000046"));
        object shellLink = Activator.CreateInstance(shellLinkType);

        ((IShellLinkW)shellLink).SetPath(targetExe);

        // 设置 AppUserModelID
        PropertyKey appIdKey = new PropertyKey { 
            fmtid = new Guid("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"), 
            pid = 5 
        };

        using (PropVariant pv = new PropVariant(appId)) {
            ((IPropertyStore)shellLink).SetValue(ref appIdKey, pv);
            ((IPropertyStore)shellLink).Commit();
        }

        // 保存文件
        ((IPersistFile)shellLink).Save(linkPath, true);
    }
}
"@

# 3. 运行
Add-Type -TypeDefinition $code
[ShellLinkHelper]::CreateShortcutWithId($shortcutPath, $targetExe, $appId)

Write-Host "---"
Write-Host "快捷方式已成功创建于: $shortcutPath" -ForegroundColor Cyan
Write-Host "AppID 已注入: $appId" -ForegroundColor Cyan