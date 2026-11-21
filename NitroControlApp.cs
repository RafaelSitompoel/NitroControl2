using System;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Net.Sockets;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Security.Principal;
using System.Threading;
using System.Windows.Forms;

namespace NitroControl
{
    static class Program
    {
        [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr h);
        [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr h, int n);
        const int SW_RESTORE = 9;
        const string APP_URL = "http://127.0.0.1:7878";
        const int PORT = 7878;

        [STAThread]
        static void Main()
        {
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);

            if (!IsAdmin())
            {
                try { Process.Start(new ProcessStartInfo { UseShellExecute = true, FileName = SelfPath(), Verb = "runas", WorkingDirectory = AppDir() }); }
                catch { }
                return;
            }

            if (IsPortInUse(PORT)) { KillPort(PORT); Thread.Sleep(700); }
            StartBackend();

            for (int i = 0; i < 35; i++) { Thread.Sleep(200); if (IsPortInUse(PORT)) break; }

            LaunchWindow();
            RunTray();
        }

        // ── Helpers ──────────────────────────────────────────────────────────
        static string AppDir()  => AppDomain.CurrentDomain.BaseDirectory;
        static string SelfPath() { try { return Process.GetCurrentProcess().MainModule!.FileName!; } catch { return Assembly.GetExecutingAssembly().Location; } }

        static bool IsAdmin()
        {
            try { using var id = WindowsIdentity.GetCurrent(); return new WindowsPrincipal(id).IsInRole(WindowsBuiltInRole.Administrator); }
            catch { return false; }
        }

        static bool IsPortInUse(int port)
        {
            try { using var c = new TcpClient(); var r = c.BeginConnect("127.0.0.1", port, null, null); bool ok = r.AsyncWaitHandle.WaitOne(400); if (ok) c.EndConnect(r); return ok; }
            catch { return false; }
        }

        static void KillPort(int port)
        {
            RunHidden("powershell",
                $"-NoProfile -Command \"Get-NetTCPConnection -LocalPort {port} -ErrorAction SilentlyContinue" +
                " | ForEach-Object {{ Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }}\"");
        }

        static void StartBackend()
        {
            // Installer places backend next to launcher
            string[] candidates = {
                Path.Combine(AppDir(), "nitrocontrol-backend.exe"),
                Path.Combine(AppDir(), "NitroControl-Desktop", "nitrocontrol-backend.exe"),
            };
            foreach (var path in candidates)
            {
                if (!File.Exists(path)) continue;
                try
                {
                    Process.Start(new ProcessStartInfo(path, "--no-window")
                    {
                        WorkingDirectory = Path.GetDirectoryName(path)!,
                        CreateNoWindow   = true,
                        UseShellExecute  = false,
                        WindowStyle      = ProcessWindowStyle.Hidden
                    });
                    return;
                }
                catch { }
            }
        }

        static Process? _appProc;

        static void LaunchWindow()
        {
            string[] browsers = {
                @"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
                @"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
                @"C:\Program Files\Google\Chrome\Application\chrome.exe",
                @"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            };
            string cache = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "NitroControl", "desktop_cache");

            foreach (var b in browsers)
            {
                if (!File.Exists(b)) continue;
                try
                {
                    _appProc = Process.Start(new ProcessStartInfo(b,
                        $"--app={APP_URL} --window-size=1360,860 " +
                        $"--user-data-dir=\"{cache}\" " +
                        "--disable-extensions --no-first-run --disable-default-apps")
                    { UseShellExecute = false });
                    return;
                }
                catch { }
            }
            Process.Start(new ProcessStartInfo(APP_URL) { UseShellExecute = true });
        }

        // ── Tray icon ────────────────────────────────────────────────────────
        static NotifyIcon? _tray;

        static void RunTray()
        {
            _tray = new NotifyIcon
            {
                Icon             = BuildIcon(),
                Text             = "NitroControl",
                Visible          = true,
                ContextMenuStrip = BuildMenu()
            };
            _tray.DoubleClick += (_, _) => OpenOrFocus();
            Application.Run();
        }

        static void OpenOrFocus()
        {
            if (_appProc is { HasExited: false })
            {
                _appProc.Refresh();
                var hw = _appProc.MainWindowHandle;
                if (hw != IntPtr.Zero) { ShowWindow(hw, SW_RESTORE); SetForegroundWindow(hw); return; }
            }
            LaunchWindow();
        }

        static ContextMenuStrip BuildMenu()
        {
            var m = new ContextMenuStrip();
            m.Items.Add("Open NitroControl", null, (_, _) => OpenOrFocus());
            m.Items.Add(new ToolStripSeparator());
            m.Items.Add("Exit", null, (_, _) =>
            {
                _tray!.Visible = false;
                KillPort(PORT);
                try { _appProc?.CloseMainWindow(); } catch { }
                Application.Exit();
            });
            return m;
        }

        // ── Icon drawn in code (fan blades, Nitro red-orange) ────────────────
        static Icon BuildIcon()
        {
            using var bmp = new Bitmap(64, 64, System.Drawing.Imaging.PixelFormat.Format32bppArgb);
            using var g   = Graphics.FromImage(bmp);
            g.SmoothingMode = System.Drawing.Drawing2D.SmoothingMode.AntiAlias;

            // Dark circle background
            g.FillEllipse(new SolidBrush(Color.FromArgb(255, 12, 12, 18)), 1, 1, 62, 62);
            // Outer ring
            g.DrawEllipse(new Pen(Color.FromArgb(255, 230, 50, 20), 4f), 3, 3, 58, 58);
            g.DrawEllipse(new Pen(Color.FromArgb(140, 255, 100, 40), 2f), 9, 9, 46, 46);

            // Three fan blades at 0°, 120°, 240°
            foreach (int deg in new[] { 0, 120, 240 })
                DrawBlade(g, 32, 32, deg);

            // Hub
            g.FillEllipse(new SolidBrush(Color.FromArgb(255, 230, 50, 20)), 25, 25, 14, 14);
            g.FillEllipse(new SolidBrush(Color.FromArgb(200, 255, 140, 80)), 28, 28,  8,  8);

            var hicon = bmp.GetHicon();
            return Icon.FromHandle(hicon);
        }

        static void DrawBlade(Graphics g, float cx, float cy, float deg)
        {
            double r = deg * Math.PI / 180.0;
            float tx = cx + (float)(Math.Cos(r) * 19), ty = cy + (float)(Math.Sin(r) * 19);
            float px = (float)(-Math.Sin(r) * 7),      py = (float)(Math.Cos(r) * 7);

            var pts = new[] {
                new PointF(cx+px*0.4f, cy+py*0.4f),
                new PointF(tx+px,      ty+py),
                new PointF(tx-px*0.3f, ty-py*0.3f),
                new PointF(cx-px*0.4f, cy-py*0.4f)
            };
            g.FillPolygon(new SolidBrush(Color.FromArgb(220, 255, 80, 20)), pts);
            g.DrawPolygon(new Pen(Color.FromArgb(160, 255, 140, 60), 1f), pts);
        }

        static void RunHidden(string exe, string args)
        {
            try { var p = Process.Start(new ProcessStartInfo(exe, args) { CreateNoWindow = true, UseShellExecute = false, WindowStyle = ProcessWindowStyle.Hidden }); p?.WaitForExit(3000); }
            catch { }
        }
    }
}
