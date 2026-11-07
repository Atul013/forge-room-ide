using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Markup;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Shell;
using System.Windows.Threading;

namespace ForgeRoom.Setup
{
    // Embedded resources (see build-ember.ps1): engine.exe, art.png, LICENSE-MIT.txt,
    // EARLY_PREVIEW_NOTES.md, meta.txt (key=value: version, payloadBytes, requiredBytes).
    internal static class Embedded
    {
        public static Stream Open(string name)
        {
            Stream stream = Assembly.GetExecutingAssembly().GetManifestResourceStream(name);
            if (stream == null) throw new InvalidOperationException("Missing embedded resource " + name);
            return stream;
        }

        public static string Text(string name)
        {
            using (StreamReader reader = new StreamReader(Open(name), Encoding.UTF8)) return reader.ReadToEnd();
        }

        public static Dictionary<string, string> Meta()
        {
            Dictionary<string, string> result = new Dictionary<string, string>();
            foreach (string line in Text("meta.txt").Split('\n'))
            {
                int eq = line.IndexOf('=');
                if (eq > 0) result[line.Substring(0, eq).Trim()] = line.Substring(eq + 1).Trim();
            }
            return result;
        }
    }

    internal static class Engine
    {
        public const string AppId = "{AFA3E0AF-AD68-47D2-8593-8B17A4A3F2CE}";

        // The Inno Setup engine does the real install/upgrade/uninstall registration.
        public static string Extract(out string workDirectory)
        {
            workDirectory = System.IO.Path.Combine(System.IO.Path.GetTempPath(), "ForgeRoomSetup-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(workDirectory);
            string exe = System.IO.Path.Combine(workDirectory, "engine-setup.exe");
            using (Stream input = Embedded.Open("engine.exe"))
            using (FileStream output = File.Create(exe)) input.CopyTo(output);
            return exe;
        }

        public static void Cleanup(string workDirectory)
        {
            try { if (workDirectory != null && Directory.Exists(workDirectory)) Directory.Delete(workDirectory, true); }
            catch (IOException) { }
            catch (UnauthorizedAccessException) { }
        }

        // The folder of an existing per-user install, so upgrades default to the same place.
        public static string ExistingInstallDirectory()
        {
            try
            {
                using (Microsoft.Win32.RegistryKey key = Microsoft.Win32.Registry.CurrentUser.OpenSubKey(
                    @"Software\Microsoft\Windows\CurrentVersion\Uninstall\" + AppId + "_is1"))
                {
                    if (key == null) return null;
                    string path = key.GetValue("Inno Setup: App Path") as string;
                    return string.IsNullOrEmpty(path) ? null : path;
                }
            }
            catch (Exception) { return null; }
        }

        // The installed version recorded by the engine, or null when Forge Room is not installed.
        public static string InstalledVersion()
        {
            try
            {
                using (Microsoft.Win32.RegistryKey key = Microsoft.Win32.Registry.CurrentUser.OpenSubKey(
                    @"Software\Microsoft\Windows\CurrentVersion\Uninstall\" + AppId + "_is1"))
                {
                    if (key == null) return null;
                    string version = key.GetValue("DisplayVersion") as string;
                    return string.IsNullOrEmpty(version) ? null : version;
                }
            }
            catch (Exception) { return null; }
        }

        public const string UninstallerName = "forge-room-uninstall.exe";

        // Put a copy of this setup in the app folder and point Windows' "Uninstall" at it, so removing
        // Forge Room shows the themed uninstaller. The Inno engine (unins000.exe) still does the work.
        public static void RegisterUninstaller(string directory)
        {
            try
            {
                string self = Assembly.GetExecutingAssembly().Location;
                string target = System.IO.Path.Combine(directory, UninstallerName);
                File.Copy(self, target, true);
                using (Microsoft.Win32.RegistryKey key = Microsoft.Win32.Registry.CurrentUser.OpenSubKey(
                    @"Software\Microsoft\Windows\CurrentVersion\Uninstall\" + AppId + "_is1", true))
                {
                    if (key != null)
                        key.SetValue("UninstallString", InstallerPolicy.Quote(target) + " /UNINSTALL", Microsoft.Win32.RegistryValueKind.String);
                }
            }
            catch (Exception) { }
        }

        // Ask a running Forge Room (from this folder) to close. False if it did not exit in time.
        public static bool CloseRunningApp(string appExe)
        {
            foreach (Process process in Process.GetProcessesByName("forge-room-early-preview"))
            {
                try
                {
                    string file = process.MainModule.FileName;
                    if (!string.Equals(file, appExe, StringComparison.OrdinalIgnoreCase)) continue;
                    process.CloseMainWindow();
                    if (!process.WaitForExit(10000)) return false;
                }
                catch (Exception) { }
            }
            return true;
        }

        // Runs the Inno uninstall engine silently. Returns null on success or a readable error.
        public static string Uninstall(string directory)
        {
            if (!CloseRunningApp(System.IO.Path.Combine(directory, "forge-room-early-preview.exe")))
                return "Forge Room is still running. Close it, then run uninstall again.";
            string engine = System.IO.Path.Combine(directory, "unins000.exe");
            if (!File.Exists(engine)) return "The uninstall engine was not found in " + directory + ".";
            try
            {
                ProcessStartInfo info = new ProcessStartInfo(engine, "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART");
                info.UseShellExecute = false;
                info.CreateNoWindow = true;
                using (Process process = Process.Start(info))
                {
                    process.WaitForExit();
                    if (process.ExitCode != 0) return "The uninstall engine stopped with code " + process.ExitCode + ".";
                }
            }
            catch (Exception ex) { return ex.Message; }
            // The engine can hand off to a temporary copy of itself; wait for the registration to go.
            for (int i = 0; i < 50 && InstalledVersion() != null; i++) System.Threading.Thread.Sleep(200);
            return InstalledVersion() == null ? null : "Uninstall did not finish.";
        }

        public static string DefaultInstallDirectory()
        {
            string existing = ExistingInstallDirectory();
            if (existing != null) return existing;
            return System.IO.Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "Programs", "Forge Room Early Preview");
        }

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern bool GetDiskFreeSpaceEx(string directory, out ulong freeToCaller, out ulong total, out ulong totalFree);

        // Free bytes on the volume that will hold the destination, or null when it cannot be read.
        public static long? FreeBytes(string destination)
        {
            try
            {
                string probe = System.IO.Path.GetFullPath(destination);
                while (!string.IsNullOrEmpty(probe) && !Directory.Exists(probe)) probe = System.IO.Path.GetDirectoryName(probe);
                if (string.IsNullOrEmpty(probe)) return null;
                ulong free, total, totalFree;
                if (!GetDiskFreeSpaceEx(probe, out free, out total, out totalFree)) return null;
                return (long)free;
            }
            catch (Exception) { return null; }
        }

        public static string VolumeLabel(string destination)
        {
            try
            {
                string root = System.IO.Path.GetPathRoot(System.IO.Path.GetFullPath(destination));
                if (!string.IsNullOrEmpty(root) && root.Length >= 2 && root[1] == ':') return root.Substring(0, 2);
                return root == null ? "destination" : root.TrimEnd('\\');
            }
            catch (Exception) { return "destination"; }
        }

        public static string BuildArguments(string logPath, string destination, bool desktopShortcut)
        {
            // /CLOSEAPPLICATIONS: ask a running Forge Room to close (Restart Manager) so an update can replace it.
            return "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /CURRENTUSER /CLOSEAPPLICATIONS" +
                   " /LOG=" + InstallerPolicy.Quote(logPath) +
                   " /DIR=" + InstallerPolicy.Quote(destination.TrimEnd('\\')) +
                   " /MERGETASKS=" + (desktopShortcut ? "desktopicon" : "!desktopicon");
        }

        // Keep the original silent command line working: re-quote /KEY=value arguments with spaces.
        public static string PassThrough(string[] args)
        {
            List<string> parts = new List<string>();
            foreach (string arg in args)
            {
                int eq = arg.IndexOf('=');
                if (arg.StartsWith("/", StringComparison.Ordinal) && eq > 0 && arg.IndexOf(' ') >= 0)
                    parts.Add(arg.Substring(0, eq + 1) + InstallerPolicy.Quote(arg.Substring(eq + 1).TrimEnd('\\')));
                else if (arg.IndexOf(' ') >= 0) parts.Add(InstallerPolicy.Quote(arg));
                else parts.Add(arg);
            }
            return string.Join(" ", parts.ToArray());
        }
    }

    internal static class Program
    {
        [STAThread]
        private static int Main(string[] args)
        {
            bool uninstall = false, fromTemp = false;
            string appDirectory = null;
            foreach (string arg in args)
            {
                if (arg.Equals("/UNINSTALL", StringComparison.OrdinalIgnoreCase)) uninstall = true;
                else if (arg.Equals("/FROMTEMP", StringComparison.OrdinalIgnoreCase)) fromTemp = true;
                else if (arg.StartsWith("/APPDIR=", StringComparison.OrdinalIgnoreCase)) appDirectory = arg.Substring(8);
            }
            if (uninstall && !fromTemp) return RelaunchUninstallFromTemp();
            if (!uninstall)
            {
                foreach (string arg in args)
                {
                    if (arg.StartsWith("/SILENT", StringComparison.OrdinalIgnoreCase) ||
                        arg.StartsWith("/VERYSILENT", StringComparison.OrdinalIgnoreCase))
                        return RunSilent(args);
                }
            }
            Application app = new Application();
            app.ShutdownMode = ShutdownMode.OnMainWindowClose;
            SetupWindow window = uninstall ? new SetupWindow(appDirectory) : new SetupWindow();
            return app.Run(window);
        }

        // The uninstaller lives in the folder it deletes, so run it from a temporary copy.
        private static int RelaunchUninstallFromTemp()
        {
            string self = Assembly.GetExecutingAssembly().Location;
            string appDirectory = System.IO.Path.GetDirectoryName(self);
            string tempDirectory = System.IO.Path.Combine(System.IO.Path.GetTempPath(), "ForgeRoomUninstall-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(tempDirectory);
            string copy = System.IO.Path.Combine(tempDirectory, "uninstall.exe");
            File.Copy(self, copy, true);
            ProcessStartInfo info = new ProcessStartInfo(copy, "/UNINSTALL /FROMTEMP /APPDIR=" + InstallerPolicy.Quote(appDirectory.TrimEnd('\\')));
            info.UseShellExecute = false;
            Process.Start(info);
            return 0;
        }

        private static int RunSilent(string[] args)
        {
            string work;
            string engine = Engine.Extract(out work);
            try
            {
                ProcessStartInfo info = new ProcessStartInfo(engine, Engine.PassThrough(args));
                info.UseShellExecute = false;
                using (Process process = Process.Start(info))
                {
                    process.WaitForExit();
                    if (process.ExitCode == 0)
                    {
                        string directory = null;
                        foreach (string arg in args)
                            if (arg.StartsWith("/DIR=", StringComparison.OrdinalIgnoreCase)) directory = arg.Substring(5).Trim('"');
                        if (string.IsNullOrEmpty(directory)) directory = Engine.ExistingInstallDirectory();
                        if (!string.IsNullOrEmpty(directory)) Engine.RegisterUninstaller(directory);
                    }
                    return process.ExitCode;
                }
            }
            finally { Engine.Cleanup(work); }
        }
    }

    internal sealed class Palette
    {
        public string Bg, Tint, Text, Muted, Disabled, Border, Field, Field2, Red, RedText, Check, Thumb, Hover;
        public bool HighContrast;

        public static Palette Create()
        {
            if (SystemParameters.HighContrast)
            {
                return new Palette
                {
                    HighContrast = true,
                    Bg = Hex(SystemColors.WindowColor), Tint = Hex(SystemColors.WindowColor),
                    Text = Hex(SystemColors.WindowTextColor), Muted = Hex(SystemColors.WindowTextColor),
                    Disabled = Hex(SystemColors.GrayTextColor), Border = Hex(SystemColors.WindowTextColor),
                    Field = Hex(SystemColors.WindowColor), Field2 = Hex(SystemColors.WindowColor),
                    Red = Hex(SystemColors.HighlightColor), RedText = Hex(SystemColors.HighlightTextColor),
                    Check = Hex(SystemColors.WindowColor), Thumb = Hex(SystemColors.WindowTextColor),
                    Hover = Hex(SystemColors.WindowTextColor)
                };
            }
            return new Palette
            {
                Bg = "#FF121314", Tint = "#BC0A0B0C", Text = "#FFEFEFEE", Muted = "#FFA0A1A5",
                Disabled = "#FF74757A", Border = "#FF34353A", Field = "#FF0D0E0F", Field2 = "#FF2A2B2E",
                Red = "#FFC74C45", RedText = "#FFFFFFFF", Check = "#FF3A3B3F", Thumb = "#FF4A4B50", Hover = "#FFFFFFFF"
            };
        }

        private static string Hex(Color c)
        {
            return "#" + c.A.ToString("X2") + c.R.ToString("X2") + c.G.ToString("X2") + c.B.ToString("X2");
        }

        public string Apply(string xaml)
        {
            return xaml.Replace("@BG@", Bg).Replace("@TEXT@", Text).Replace("@MUTED@", Muted)
                .Replace("@DISABLED@", Disabled).Replace("@BORDER@", Border).Replace("@FIELD2@", Field2)
                .Replace("@FIELD@", Field).Replace("@REDTEXT@", RedText).Replace("@RED@", Red)
                .Replace("@CHECK@", Check).Replace("@THUMB@", Thumb).Replace("@HOVER@", Hover);
        }
    }

    internal static class Styles
    {
        public const string Xaml = @"
<ResourceDictionary xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
  <Style x:Key='Btn' TargetType='Button'>
    <Setter Property='Foreground' Value='@TEXT@'/>
    <Setter Property='Background' Value='@FIELD2@'/>
    <Setter Property='BorderBrush' Value='@BORDER@'/>
    <Setter Property='BorderThickness' Value='1'/>
    <Setter Property='FontSize' Value='14'/>
    <Setter Property='Height' Value='34'/>
    <Setter Property='MinWidth' Value='92'/>
    <Setter Property='Cursor' Value='Hand'/>
    <Setter Property='Template'>
      <Setter.Value>
        <ControlTemplate TargetType='Button'>
          <Border x:Name='bd' Background='{TemplateBinding Background}' BorderBrush='{TemplateBinding BorderBrush}'
                  BorderThickness='{TemplateBinding BorderThickness}' CornerRadius='4'>
            <Grid>
              <Border x:Name='ov' CornerRadius='3' Background='@HOVER@' Opacity='0'/>
              <ContentPresenter HorizontalAlignment='Center' VerticalAlignment='Center' Margin='14,0'/>
            </Grid>
          </Border>
          <ControlTemplate.Triggers>
            <Trigger Property='IsMouseOver' Value='True'><Setter TargetName='ov' Property='Opacity' Value='0.08'/></Trigger>
            <Trigger Property='IsPressed' Value='True'><Setter TargetName='ov' Property='Opacity' Value='0.14'/></Trigger>
            <Trigger Property='IsKeyboardFocused' Value='True'>
              <Setter TargetName='bd' Property='BorderBrush' Value='@TEXT@'/>
            </Trigger>
          </ControlTemplate.Triggers>
        </ControlTemplate>
      </Setter.Value>
    </Setter>
    <Style.Triggers>
      <Trigger Property='IsEnabled' Value='False'><Setter Property='Foreground' Value='@DISABLED@'/><Setter Property='Cursor' Value='Arrow'/></Trigger>
    </Style.Triggers>
  </Style>

  <Style x:Key='PrimaryBtn' TargetType='Button' BasedOn='{StaticResource Btn}'>
    <Setter Property='Background' Value='@RED@'/>
    <Setter Property='BorderBrush' Value='@RED@'/>
    <Setter Property='Foreground' Value='@REDTEXT@'/>
    <Setter Property='FontWeight' Value='SemiBold'/>
    <Style.Triggers>
      <Trigger Property='IsEnabled' Value='False'>
        <Setter Property='Background' Value='@FIELD2@'/>
        <Setter Property='BorderBrush' Value='@BORDER@'/>
        <Setter Property='Foreground' Value='@DISABLED@'/>
      </Trigger>
    </Style.Triggers>
  </Style>

  <Style x:Key='LinkBtn' TargetType='Button'>
    <Setter Property='Foreground' Value='@MUTED@'/>
    <Setter Property='FontSize' Value='14'/>
    <Setter Property='Cursor' Value='Hand'/>
    <Setter Property='Template'>
      <Setter.Value>
        <ControlTemplate TargetType='Button'>
          <Border x:Name='bd' Background='Transparent' BorderBrush='Transparent' BorderThickness='1' CornerRadius='3' Padding='4,2'>
            <ContentPresenter/>
          </Border>
          <ControlTemplate.Triggers>
            <Trigger Property='IsMouseOver' Value='True'><Setter Property='Foreground' Value='@TEXT@'/></Trigger>
            <Trigger Property='IsKeyboardFocused' Value='True'><Setter TargetName='bd' Property='BorderBrush' Value='@TEXT@'/></Trigger>
          </ControlTemplate.Triggers>
        </ControlTemplate>
      </Setter.Value>
    </Setter>
  </Style>

  <Style x:Key='Caption' TargetType='Button'>
    <Setter Property='Foreground' Value='@TEXT@'/>
    <Setter Property='FontFamily' Value='Segoe MDL2 Assets'/>
    <Setter Property='FontSize' Value='10'/>
    <Setter Property='Width' Value='46'/>
    <Setter Property='Height' Value='36'/>
    <Setter Property='Focusable' Value='False'/>
    <Setter Property='Template'>
      <Setter.Value>
        <ControlTemplate TargetType='Button'>
          <Border x:Name='bd' Background='Transparent'><ContentPresenter HorizontalAlignment='Center' VerticalAlignment='Center'/></Border>
          <ControlTemplate.Triggers>
            <Trigger Property='IsMouseOver' Value='True'><Setter TargetName='bd' Property='Background' Value='#33808080'/></Trigger>
          </ControlTemplate.Triggers>
        </ControlTemplate>
      </Setter.Value>
    </Setter>
  </Style>

  <Style x:Key='CaptionClose' TargetType='Button' BasedOn='{StaticResource Caption}'>
    <Setter Property='Template'>
      <Setter.Value>
        <ControlTemplate TargetType='Button'>
          <Border x:Name='bd' Background='Transparent'><ContentPresenter HorizontalAlignment='Center' VerticalAlignment='Center'/></Border>
          <ControlTemplate.Triggers>
            <Trigger Property='IsMouseOver' Value='True'><Setter TargetName='bd' Property='Background' Value='#C42B1C'/></Trigger>
          </ControlTemplate.Triggers>
        </ControlTemplate>
      </Setter.Value>
    </Setter>
  </Style>

  <Style TargetType='CheckBox'>
    <Setter Property='Foreground' Value='@TEXT@'/>
    <Setter Property='FontSize' Value='14'/>
    <Setter Property='Cursor' Value='Hand'/>
    <Setter Property='Template'>
      <Setter.Value>
        <ControlTemplate TargetType='CheckBox'>
          <StackPanel Orientation='Horizontal' Background='Transparent'>
            <Border x:Name='bx' Width='18' Height='18' CornerRadius='3' BorderThickness='1' BorderBrush='@MUTED@'
                    Background='@FIELD@' VerticalAlignment='Center'>
              <Path x:Name='ck' Data='M4,9 L7.5,12.5 L14,5.5' Stroke='@TEXT@' StrokeThickness='2'
                    StrokeStartLineCap='Round' StrokeEndLineCap='Round' StrokeLineJoin='Round' Visibility='Collapsed'/>
            </Border>
            <ContentPresenter Margin='10,0,0,0' VerticalAlignment='Center'/>
          </StackPanel>
          <ControlTemplate.Triggers>
            <Trigger Property='IsChecked' Value='True'>
              <Setter TargetName='ck' Property='Visibility' Value='Visible'/>
              <Setter TargetName='bx' Property='Background' Value='@CHECK@'/>
              <Setter TargetName='bx' Property='BorderBrush' Value='@TEXT@'/>
            </Trigger>
            <Trigger Property='IsMouseOver' Value='True'><Setter TargetName='bx' Property='BorderBrush' Value='@TEXT@'/></Trigger>
            <Trigger Property='IsKeyboardFocused' Value='True'><Setter TargetName='bx' Property='BorderThickness' Value='2'/><Setter TargetName='bx' Property='BorderBrush' Value='@TEXT@'/></Trigger>
          </ControlTemplate.Triggers>
        </ControlTemplate>
      </Setter.Value>
    </Setter>
  </Style>

  <Style TargetType='ScrollBar'>
    <Setter Property='Width' Value='10'/>
    <Setter Property='Background' Value='Transparent'/>
    <Setter Property='Template'>
      <Setter.Value>
        <ControlTemplate TargetType='ScrollBar'>
          <Grid Background='Transparent'>
            <Track x:Name='PART_Track' IsDirectionReversed='True'>
              <Track.DecreaseRepeatButton>
                <RepeatButton Command='ScrollBar.PageUpCommand' Focusable='False'>
                  <RepeatButton.Template><ControlTemplate TargetType='RepeatButton'><Border Background='Transparent'/></ControlTemplate></RepeatButton.Template>
                </RepeatButton>
              </Track.DecreaseRepeatButton>
              <Track.IncreaseRepeatButton>
                <RepeatButton Command='ScrollBar.PageDownCommand' Focusable='False'>
                  <RepeatButton.Template><ControlTemplate TargetType='RepeatButton'><Border Background='Transparent'/></ControlTemplate></RepeatButton.Template>
                </RepeatButton>
              </Track.IncreaseRepeatButton>
              <Track.Thumb>
                <Thumb>
                  <Thumb.Template><ControlTemplate TargetType='Thumb'><Border Margin='2,0' CornerRadius='3' Background='@THUMB@'/></ControlTemplate></Thumb.Template>
                </Thumb>
              </Track.Thumb>
            </Track>
          </Grid>
        </ControlTemplate>
      </Setter.Value>
    </Setter>
  </Style>

  <Style TargetType='ProgressBar'>
    <Setter Property='Height' Value='6'/>
    <Setter Property='Template'>
      <Setter.Value>
        <ControlTemplate TargetType='ProgressBar'>
          <Grid>
            <Border Background='@FIELD2@' CornerRadius='3'/>
            <Border x:Name='PART_Track'>
              <Border x:Name='PART_Indicator' HorizontalAlignment='Left' Background='@RED@' CornerRadius='3'/>
            </Border>
          </Grid>
        </ControlTemplate>
      </Setter.Value>
    </Setter>
  </Style>
</ResourceDictionary>";
    }

    internal enum Page { Welcome, License, Options, Progress, Finish }

    internal sealed class SetupWindow : Window
    {
        private const int WindowWidth = 880;
        private const int WindowHeight = 564;
        private const int TitleHeight = 36;
        private const int FooterHeight = 58;

        private readonly Palette palette = Palette.Create();
        private readonly Dictionary<string, string> meta = Embedded.Meta();
        private readonly long payloadBytes;
        private readonly long requiredBytes;

        private readonly string installedVersion;
        private readonly UpgradeKind kind;
        private Brush tintBrush;
        private Grid[] pages = new Grid[5];
        private Page current = Page.Welcome;

        private Button backButton, primaryButton, cancelButton;
        private TextBlock footerNote;

        private CheckBox acceptCheck, desktopCheck, launchCheck;
        private TextBlock acceptHelp, pathMessage, availableText, availableLabel, licenseAccepted;
        private TextBox pathBox;
        private ProgressBar progress;
        private TextBlock progressStatus, finishHeading, finishSubtitle, finishDetail;

        private DispatcherTimer timer;
        private Process engineProcess;
        private string engineWork, engineExe, logPath;
        private bool installing, succeeded;
        private string installedDirectory;

        private readonly string uninstallDirectory;
        private bool Uninstalling { get { return uninstallDirectory != null; } }

        public SetupWindow() : this(null) { }

        public SetupWindow(string uninstallDirectory)
        {
            this.uninstallDirectory = uninstallDirectory;
            payloadBytes = long.Parse(meta["payloadBytes"]);
            requiredBytes = InstallerPolicy.RequiredBytes(meta["requiredText"]) ?? payloadBytes;

            ResourceDictionary dictionary = (ResourceDictionary)XamlReader.Parse(palette.Apply(Styles.Xaml));
            Resources.MergedDictionaries.Add(dictionary);

            Title = "Forge Room Setup";
            Width = WindowWidth;
            Height = WindowHeight;
            ResizeMode = ResizeMode.NoResize;
            WindowStyle = WindowStyle.None;
            WindowStartupLocation = WindowStartupLocation.Manual;
            Background = Brushes.Transparent;
            FontFamily = new FontFamily("Segoe UI");
            UseLayoutRounding = true;
            SnapsToDevicePixels = true;
            TextOptions.SetTextFormattingMode(this, TextFormattingMode.Display);

            WindowChrome.SetWindowChrome(this, new WindowChrome
            {
                CaptionHeight = TitleHeight,
                ResizeBorderThickness = new Thickness(0),
                GlassFrameThickness = new Thickness(-1),
                UseAeroCaptionButtons = false,
                CornerRadius = new CornerRadius(0)
            });

            installedVersion = Engine.InstalledVersion();
            kind = InstallerPolicy.Classify(installedVersion, meta["version"]);
            tintBrush = palette.HighContrast ? Brush(palette.Bg) : GlassSheen();
            Content = BuildRoot();
            ShowPage(Page.Welcome);

            Closing += OnClosing;
        }

        // ---- Window material -------------------------------------------------------------

        [DllImport("dwmapi.dll")]
        private static extern int DwmSetWindowAttribute(IntPtr hwnd, int attribute, ref int value, int size);

        protected override void OnSourceInitialized(EventArgs e)
        {
            base.OnSourceInitialized(e);
            Rect area = SystemParameters.WorkArea;
            Left = area.Left + Math.Max(0, (area.Width - Width) / 2);
            Top = area.Top + Math.Max(0, (area.Height - Height) / 2);
            IntPtr handle = new WindowInteropHelper(this).Handle;
            HwndSource source = HwndSource.FromHwnd(handle);
            if (source != null && source.CompositionTarget != null)
                source.CompositionTarget.BackgroundColor = Colors.Transparent;

            int dark = 1, round = 2, none = 1;
            DwmSetWindowAttribute(handle, 20, ref dark, 4);     // immersive dark mode
            DwmSetWindowAttribute(handle, 33, ref round, 4);    // rounded corners
            DwmSetWindowAttribute(handle, 38, ref none, 4);     // no system backdrop: we set our own blur and tint
            // Acrylic blur-behind with our own tint (smoky #121314, ~60% opaque). Unlike the system
            // backdrop this keeps its blur when the window is inactive and does not add a gray veil.
            if (palette.HighContrast || !EnableAcrylic(handle, 0x99, 0x12, 0x13, 0x14))
            {
                // Unsupported or high contrast: readable opaque surface instead.
                tintBrush = Brush(palette.Bg);
                ApplyTint();
            }
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct AccentPolicy
        {
            public int AccentState;
            public int AccentFlags;
            public int GradientColor;   // AABBGGRR
            public int AnimationId;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct WindowCompositionAttributeData
        {
            public int Attribute;
            public IntPtr Data;
            public int SizeOfData;
        }

        [DllImport("user32.dll")]
        private static extern int SetWindowCompositionAttribute(IntPtr hwnd, ref WindowCompositionAttributeData data);

        private static bool EnableAcrylic(IntPtr hwnd, byte alpha, byte r, byte g, byte b)
        {
            AccentPolicy policy = new AccentPolicy();
            policy.AccentState = 4; // ACCENT_ENABLE_ACRYLICBLURBEHIND
            policy.AccentFlags = 2;
            policy.GradientColor = (alpha << 24) | (b << 16) | (g << 8) | r;
            int size = Marshal.SizeOf(typeof(AccentPolicy));
            IntPtr buffer = Marshal.AllocHGlobal(size);
            try
            {
                Marshal.StructureToPtr(policy, buffer, false);
                WindowCompositionAttributeData data = new WindowCompositionAttributeData();
                data.Attribute = 19; // WCA_ACCENT_POLICY
                data.Data = buffer;
                data.SizeOfData = size;
                return SetWindowCompositionAttribute(hwnd, ref data) != 0;
            }
            catch (Exception) { return false; }
            finally { Marshal.FreeHGlobal(buffer); }
        }

        // Faint top-left sheen so the pane still reads as glass over a plain backdrop.
        private static Brush GlassSheen()
        {
            LinearGradientBrush brush = new LinearGradientBrush();
            brush.StartPoint = new Point(0, 0);
            brush.EndPoint = new Point(1, 1);
            brush.GradientStops.Add(new GradientStop(Color.FromArgb(0x16, 0xFF, 0xFF, 0xFF), 0.0));
            brush.GradientStops.Add(new GradientStop(Color.FromArgb(0x04, 0xFF, 0xFF, 0xFF), 0.55));
            brush.GradientStops.Add(new GradientStop(Color.FromArgb(0x00, 0xFF, 0xFF, 0xFF), 1.0));
            brush.Freeze();
            return brush;
        }

        private readonly List<Panel> tinted = new List<Panel>();

        private void ApplyTint()
        {
            foreach (Panel panel in tinted) panel.Background = tintBrush;
        }

        // ---- Helpers ---------------------------------------------------------------------

        private static SolidColorBrush Brush(string hex)
        {
            SolidColorBrush brush = (SolidColorBrush)new BrushConverter().ConvertFromString(hex);
            brush.Freeze();
            return brush;
        }

        private TextBlock Text(string text, double size, string color, FontWeight? weight = null)
        {
            TextBlock block = new TextBlock();
            block.Text = text;
            block.FontSize = size;
            block.Foreground = Brush(color);
            block.TextWrapping = TextWrapping.Wrap;
            if (weight.HasValue) block.FontWeight = weight.Value;
            return block;
        }

        private Button MakeButton(string text, string style)
        {
            Button button = new Button();
            button.Content = text;
            button.Style = (Style)Resources[style];
            AutomationProperties.SetName(button, text);
            return button;
        }

        private void Heading(Grid page, string title, string subtitle)
        {
            page.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            StackPanel stack = new StackPanel();
            TextBlock heading = Text(title, 26, palette.Text, FontWeights.SemiBold);
            heading.FontFamily = new FontFamily("Segoe UI Semibold");
            stack.Children.Add(heading);
            TextBlock sub = Text(subtitle, 16, palette.Muted);
            sub.Margin = new Thickness(0, 4, 0, 0);
            stack.Children.Add(sub);
            stack.Margin = new Thickness(0, 0, 0, 22);
            Grid.SetRow(stack, 0);
            page.Children.Add(stack);
            page.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        }

        // ---- Layout ----------------------------------------------------------------------

        private UIElement BuildRoot()
        {
            Grid root = new Grid();
            root.RowDefinitions.Add(new RowDefinition { Height = new GridLength(TitleHeight) });
            root.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });

            Grid titleBar = new Grid { Background = tintBrush };
            tinted.Add(titleBar);
            StackPanel titleLeft = new StackPanel { Orientation = Orientation.Horizontal, Margin = new Thickness(12, 0, 0, 0), VerticalAlignment = VerticalAlignment.Center };
            titleLeft.Children.Add(BuildLogo(20));
            TextBlock titleText = Text("Forge Room Setup", 13, palette.Text);
            titleText.Margin = new Thickness(10, 0, 0, 0);
            titleText.VerticalAlignment = VerticalAlignment.Center;
            titleLeft.Children.Add(titleText);
            titleBar.Children.Add(titleLeft);

            StackPanel caption = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right };
            Button minimize = MakeButton("\uE921", "Caption");
            AutomationProperties.SetName(minimize, "Minimize");
            minimize.Click += delegate { WindowState = WindowState.Minimized; };
            Button close = MakeButton("\uE8BB", "CaptionClose");
            AutomationProperties.SetName(close, "Close");
            close.Click += delegate { Close(); };
            WindowChrome.SetIsHitTestVisibleInChrome(minimize, true);
            WindowChrome.SetIsHitTestVisibleInChrome(close, true);
            caption.Children.Add(minimize);
            caption.Children.Add(close);
            titleBar.Children.Add(caption);
            Grid.SetRow(titleBar, 0);
            root.Children.Add(titleBar);

            Grid body = new Grid();
            body.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(238) });
            body.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            body.Children.Add(BuildBrandPanel());
            UIElement right = BuildRightPane();
            Grid.SetColumn(right, 1);
            body.Children.Add(right);
            Grid.SetRow(body, 1);
            root.Children.Add(body);
            return root;
        }

        private UIElement BuildLogo(double size)
        {
            Canvas canvas = new Canvas { Width = size, Height = size };
            System.Windows.Shapes.Ellipse ring = new System.Windows.Shapes.Ellipse
            {
                Width = size - 2, Height = size - 2, Stroke = Brush("#FFC74C45"), StrokeThickness = 1.8
            };
            Canvas.SetLeft(ring, 1);
            Canvas.SetTop(ring, 1);
            canvas.Children.Add(ring);
            double mid = size / 2, arm = size * 0.22;
            canvas.Children.Add(new System.Windows.Shapes.Line { X1 = mid - arm, X2 = mid + arm, Y1 = mid, Y2 = mid, Stroke = Brush(palette.Text), StrokeThickness = 1.8, StrokeStartLineCap = PenLineCap.Round, StrokeEndLineCap = PenLineCap.Round });
            canvas.Children.Add(new System.Windows.Shapes.Line { X1 = mid, X2 = mid, Y1 = mid - arm, Y2 = mid + arm, Stroke = Brush(palette.Text), StrokeThickness = 1.8, StrokeStartLineCap = PenLineCap.Round, StrokeEndLineCap = PenLineCap.Round });
            return canvas;
        }

        // The left panel is deliberately a flat, fully opaque surface: it never shows the backdrop.
        private UIElement BuildBrandPanel()
        {
            Grid panel = new Grid { Background = Brush("#FF121314"), ClipToBounds = true };
            BitmapImage art = new BitmapImage();
            art.BeginInit();
            art.CacheOption = BitmapCacheOption.OnLoad;
            art.StreamSource = Embedded.Open("art.png");
            art.EndInit();
            art.Freeze();
            Image image = new Image { Source = art, Stretch = Stretch.UniformToFill, VerticalAlignment = VerticalAlignment.Top };
            RenderOptions.SetBitmapScalingMode(image, BitmapScalingMode.HighQuality);
            panel.Children.Add(image);
            TextBlock version = Text(meta["version"] + "  \u00B7  Windows x64", 11, "#FFA0A1A5");
            version.VerticalAlignment = VerticalAlignment.Bottom;
            version.Margin = new Thickness(20, 0, 0, 16);
            panel.Children.Add(version);
            return panel;
        }

        private UIElement BuildRightPane()
        {
            Grid pane = new Grid { Background = tintBrush };
            tinted.Add(pane);
            pane.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            pane.RowDefinitions.Add(new RowDefinition { Height = new GridLength(FooterHeight) });

            Grid content = new Grid { Margin = new Thickness(36, 30, 36, 0) };
            pages[(int)Page.Welcome] = BuildWelcome();
            pages[(int)Page.License] = BuildLicense();
            pages[(int)Page.Options] = BuildOptions();
            pages[(int)Page.Progress] = BuildProgress();
            pages[(int)Page.Finish] = BuildFinish();
            foreach (Grid page in pages) content.Children.Add(page);
            pane.Children.Add(content);

            Border footer = new Border { BorderBrush = Brush(palette.Border), BorderThickness = new Thickness(0, 1, 0, 0) };
            Grid.SetRow(footer, 1);
            Grid footerGrid = new Grid { Margin = new Thickness(36, 0, 36, 0) };
            footerNote = Text("Windows 11  \u00B7  x64", 12, palette.Muted);
            footerNote.VerticalAlignment = VerticalAlignment.Center;
            footerNote.HorizontalAlignment = HorizontalAlignment.Left;
            footerGrid.Children.Add(footerNote);

            StackPanel buttons = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right, VerticalAlignment = VerticalAlignment.Center };
            backButton = MakeButton("Back", "Btn");
            primaryButton = MakeButton("Continue", "PrimaryBtn");
            cancelButton = MakeButton("Cancel", "Btn");
            primaryButton.IsDefault = true;
            cancelButton.IsCancel = true;
            primaryButton.Margin = new Thickness(8, 0, 0, 0);
            cancelButton.Margin = new Thickness(8, 0, 0, 0);
            backButton.Click += delegate { Back(); };
            primaryButton.Click += delegate { Primary(); };
            cancelButton.Click += delegate { Close(); };
            buttons.Children.Add(backButton);
            buttons.Children.Add(primaryButton);
            buttons.Children.Add(cancelButton);
            footerGrid.Children.Add(buttons);
            footer.Child = footerGrid;
            pane.Children.Add(footer);
            return pane;
        }

        // ---- Pages -----------------------------------------------------------------------

        private Grid BuildWelcome()
        {
            Grid page = new Grid();
            string title = "Welcome to Forge Room", subtitle = "Early Preview setup";
            string intro = "This wizard will install Forge Room on your computer.";
            string detail = "A native preview for opening, editing and saving text files.";
            if (kind == UpgradeKind.Update)
            {
                title = "Update Forge Room";
                subtitle = "Version " + installedVersion + "  →  " + meta["version"];
                intro = "This wizard will update Forge Room on your computer.";
                detail = "Your files and settings are kept. Forge Room is closed during the update.";
            }
            else if (kind == UpgradeKind.Reinstall)
            {
                subtitle = "Version " + installedVersion + " is already installed";
                intro = "This wizard will reinstall Forge Room on your computer.";
                detail = "Your files and settings are kept.";
            }
            else if (kind == UpgradeKind.Downgrade)
            {
                subtitle = "A newer version is installed";
                intro = "Forge Room " + installedVersion + " is installed. This setup contains the older version " + meta["version"] + ".";
                detail = "Setup will not replace a newer version with an older one. Close setup, or uninstall Forge Room first.";
            }
            if (Uninstalling)
            {
                title = "Remove Forge Room";
                subtitle = installedVersion == null ? "Early Preview" : "Version " + installedVersion;
                intro = "This will remove Forge Room from your computer.";
                detail = "Files you created or edited with Forge Room are not touched.";
            }
            Heading(page, title, subtitle);
            StackPanel body = new StackPanel();
            body.Children.Add(Text(intro, 14, palette.Text));
            TextBlock second = Text(detail, 14, palette.Text);
            second.Margin = new Thickness(0, 12, 0, 0);
            body.Children.Add(second);

            Border note = new Border
            {
                BorderBrush = Brush(palette.Border), BorderThickness = new Thickness(1), CornerRadius = new CornerRadius(4),
                Padding = new Thickness(14, 12, 14, 12), Margin = new Thickness(0, 28, 0, 0)
            };
            StackPanel noteRow = new StackPanel { Orientation = Orientation.Horizontal };
            TextBlock icon = Text("\uE946", 16, palette.Muted);
            icon.FontFamily = new FontFamily("Segoe MDL2 Assets");
            icon.VerticalAlignment = VerticalAlignment.Center;
            noteRow.Children.Add(icon);
            TextBlock noteText = Text(Uninstalling ? "Forge Room is closed first if it is running. Save your work before continuing."
                : "This preview is unsigned. Some features are still in development.", 14, palette.Text);
            noteText.Margin = new Thickness(12, 0, 0, 0);
            noteRow.Children.Add(noteText);
            note.Child = noteRow;
            body.Children.Add(note);

            Button notes = MakeButton("Preview notes", "LinkBtn");
            notes.HorizontalAlignment = HorizontalAlignment.Left;
            notes.Margin = new Thickness(0, 16, 0, 0);
            notes.Click += delegate { ShowNotes(); };
            if (Uninstalling) notes.Visibility = Visibility.Collapsed;
            body.Children.Add(notes);

            Grid.SetRow(body, 1);
            page.Children.Add(body);
            return page;
        }

        private Grid BuildLicense()
        {
            Grid page = new Grid();
            Heading(page, "License agreement", "Read the terms before continuing.");
            Grid body = new Grid();
            body.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            body.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });

            Border frame = new Border { BorderBrush = Brush(palette.Border), BorderThickness = new Thickness(1), CornerRadius = new CornerRadius(4), Background = Brush(palette.Field) };
            TextBox license = new TextBox
            {
                Text = Embedded.Text("LICENSE-MIT.txt").Replace("\r\n", "\n").Replace("\n\n", "\u0001").Replace("\n", " ").Replace("\u0001", "\n\n"),
                IsReadOnly = true, TextWrapping = TextWrapping.Wrap, BorderThickness = new Thickness(0),
                Background = Brushes.Transparent, Foreground = Brush(palette.Text), FontSize = 14,
                Padding = new Thickness(14, 10, 8, 10), VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
                CaretBrush = Brush(palette.Text), SelectionBrush = Brush(palette.Thumb)
            };
            AutomationProperties.SetName(license, "MIT license text");
            frame.Child = license;
            body.Children.Add(frame);

            StackPanel accept = new StackPanel { Margin = new Thickness(0, 14, 0, 0) };
            acceptCheck = new CheckBox { Content = "I accept the license agreement" };
            acceptCheck.Checked += delegate { OnAcceptChanged(); };
            acceptCheck.Unchecked += delegate { OnAcceptChanged(); };
            accept.Children.Add(acceptCheck);
            acceptHelp = Text("Accept the agreement to continue.", 12, palette.Muted);
            acceptHelp.Margin = new Thickness(28, 4, 0, 0);
            accept.Children.Add(acceptHelp);
            Grid.SetRow(accept, 1);
            body.Children.Add(accept);

            Grid.SetRow(body, 1);
            page.Children.Add(body);
            return page;
        }

        private Grid BuildOptions()
        {
            Grid page = new Grid();
            Heading(page, kind == UpgradeKind.Fresh ? "Ready to install" : "Ready to " + PrimaryLabel.ToLowerInvariant(),
                kind == UpgradeKind.Fresh ? "Choose your installation options." : "Forge Room stays in its current folder.");
            StackPanel body = new StackPanel();
            body.Children.Add(Text("Install location", 14, palette.Text));

            Border row = new Border { BorderBrush = Brush(palette.Border), BorderThickness = new Thickness(1), CornerRadius = new CornerRadius(4), Height = 36, Margin = new Thickness(0, 8, 0, 0), Background = Brush(palette.Field) };
            Grid rowGrid = new Grid();
            rowGrid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            rowGrid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            rowGrid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            TextBlock folder = Text("\uE838", 14, palette.Muted);
            folder.FontFamily = new FontFamily("Segoe MDL2 Assets");
            folder.VerticalAlignment = VerticalAlignment.Center;
            folder.Margin = new Thickness(12, 0, 4, 0);
            rowGrid.Children.Add(folder);
            pathBox = new TextBox
            {
                BorderThickness = new Thickness(0), Background = Brushes.Transparent, Foreground = Brush(palette.Text), FontSize = 14,
                VerticalContentAlignment = VerticalAlignment.Center, CaretBrush = Brush(palette.Text), Margin = new Thickness(4, 0, 4, 0),
                Text = Engine.DefaultInstallDirectory(), SelectionBrush = Brush(palette.Thumb)
            };
            AutomationProperties.SetName(pathBox, "Install location");
            pathBox.TextChanged += delegate { RefreshOptions(); };
            Grid.SetColumn(pathBox, 1);
            rowGrid.Children.Add(pathBox);
            Button change = MakeButton("Change", "LinkBtn");
            change.Margin = new Thickness(0, 0, 6, 0);
            change.VerticalAlignment = VerticalAlignment.Center;
            change.Click += delegate { Browse(); };
            if (kind != UpgradeKind.Fresh)
            {
                // Updating always targets the existing install so there is never a second copy.
                pathBox.IsReadOnly = true;
                change.Visibility = Visibility.Collapsed;
            }
            Grid.SetColumn(change, 2);
            rowGrid.Children.Add(change);
            row.Child = rowGrid;
            body.Children.Add(row);

            pathMessage = Text(kind == UpgradeKind.Fresh ? "Installs for your account." : "Updates your existing installation.", 12, palette.Muted);
            pathMessage.Margin = new Thickness(0, 6, 0, 0);
            body.Children.Add(pathMessage);

            desktopCheck = new CheckBox { Content = "Create a desktop shortcut", Margin = new Thickness(0, 16, 0, 0) };
            body.Children.Add(desktopCheck);

            Border rule = new Border { BorderBrush = Brush(palette.Border), BorderThickness = new Thickness(0, 1, 0, 0), Margin = new Thickness(0, 20, 0, 0) };
            Grid space = new Grid { Margin = new Thickness(0, 14, 0, 0) };
            space.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            space.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            space.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            space.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            TextBlock filesLabel = Text("App files", 14, palette.Text);
            TextBlock filesValue = Text(InstallerPolicy.FormatSize(payloadBytes), 14, palette.Text);
            availableLabel = Text("Available on C:", 14, palette.Text);
            availableLabel.Margin = new Thickness(0, 6, 0, 0);
            availableText = Text("", 14, palette.Text);
            availableText.Margin = new Thickness(0, 6, 0, 0);
            Grid.SetColumn(filesValue, 1);
            Grid.SetRow(availableLabel, 1);
            Grid.SetRow(availableText, 1);
            Grid.SetColumn(availableText, 1);
            space.Children.Add(filesLabel);
            space.Children.Add(filesValue);
            space.Children.Add(availableLabel);
            space.Children.Add(availableText);
            rule.Child = space;
            body.Children.Add(rule);

            TextBlock check = Text("Final space required is checked before installation.", 12, palette.Muted);
            check.Margin = new Thickness(0, 8, 0, 0);
            body.Children.Add(check);

            StackPanel accepted = new StackPanel { Orientation = Orientation.Horizontal, Margin = new Thickness(0, 14, 0, 0) };
            TextBlock tick = Text("\uE73E", 13, palette.Muted);
            tick.FontFamily = new FontFamily("Segoe MDL2 Assets");
            tick.VerticalAlignment = VerticalAlignment.Center;
            accepted.Children.Add(tick);
            licenseAccepted = Text("License accepted", 14, palette.Muted);
            licenseAccepted.Margin = new Thickness(8, 0, 0, 0);
            accepted.Children.Add(licenseAccepted);
            body.Children.Add(accepted);

            Grid.SetRow(body, 1);
            page.Children.Add(body);
            return page;
        }

        private Grid BuildProgress()
        {
            Grid page = new Grid();
            Heading(page, Uninstalling ? "Removing Forge Room" : kind == UpgradeKind.Fresh ? "Installing Forge Room" : (kind == UpgradeKind.Update ? "Updating Forge Room" : "Reinstalling Forge Room"), "This usually takes a few seconds.");
            StackPanel body = new StackPanel { Margin = new Thickness(0, 40, 0, 0) };
            progress = new ProgressBar { Minimum = 0, Maximum = 100, Value = 0 };
            AutomationProperties.SetName(progress, "Installation progress");
            body.Children.Add(progress);
            progressStatus = Text("Preparing\u2026", 14, palette.Muted);
            progressStatus.Margin = new Thickness(0, 12, 0, 0);
            body.Children.Add(progressStatus);
            Grid.SetRow(body, 1);
            page.Children.Add(body);
            return page;
        }

        private Grid BuildFinish()
        {
            Grid page = new Grid();
            page.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            page.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            StackPanel head = new StackPanel { Margin = new Thickness(0, 0, 0, 22) };
            finishHeading = Text("Forge Room is installed", 26, palette.Text, FontWeights.SemiBold);
            finishHeading.FontFamily = new FontFamily("Segoe UI Semibold");
            head.Children.Add(finishHeading);
            finishSubtitle = Text("Setup completed successfully.", 16, palette.Muted);
            finishSubtitle.Margin = new Thickness(0, 4, 0, 0);
            head.Children.Add(finishSubtitle);
            page.Children.Add(head);

            StackPanel body = new StackPanel();
            finishDetail = Text("", 14, palette.Muted);
            finishDetail.Margin = new Thickness(0, 0, 0, 20);
            body.Children.Add(finishDetail);
            launchCheck = new CheckBox { Content = "Launch Forge Room", IsChecked = true };
            body.Children.Add(launchCheck);
            Grid.SetRow(body, 1);
            page.Children.Add(body);
            return page;
        }

        // ---- Flow ------------------------------------------------------------------------

        private void ShowPage(Page page)
        {
            current = page;
            for (int i = 0; i < pages.Length; i++) pages[i].Visibility = (i == (int)page) ? Visibility.Visible : Visibility.Collapsed;

            backButton.Visibility = Visibility.Visible;
            cancelButton.Visibility = Visibility.Visible;
            cancelButton.IsEnabled = true;
            backButton.IsEnabled = true;
            footerNote.Visibility = Visibility.Collapsed;
            primaryButton.IsEnabled = true;

            switch (page)
            {
                case Page.Welcome:
                    backButton.Visibility = Visibility.Collapsed;
                    footerNote.Visibility = Uninstalling ? Visibility.Collapsed : Visibility.Visible;
                    SetPrimary(Uninstalling ? "Remove" : "Continue");
                    if (kind == UpgradeKind.Downgrade && !Uninstalling) primaryButton.IsEnabled = false;
                    break;
                case Page.License:
                    SetPrimary("Continue");
                    OnAcceptChanged();
                    break;
                case Page.Options:
                    SetPrimary(PrimaryLabel);
                    RefreshOptions();
                    break;
                case Page.Progress:
                    backButton.Visibility = Visibility.Collapsed;
                    cancelButton.IsEnabled = false;
                    primaryButton.IsEnabled = false;
                    SetPrimary(PrimaryLabel);
                    break;
                case Page.Finish:
                    backButton.Visibility = Visibility.Collapsed;
                    cancelButton.Visibility = Visibility.Collapsed;
                    SetPrimary("Finish");
                    break;
            }
            if (primaryButton.IsEnabled) primaryButton.Focus();
            else if (page == Page.License) acceptCheck.Focus();
        }

        private string PrimaryLabel
        {
            get
            {
                if (kind == UpgradeKind.Update) return "Update";
                if (kind == UpgradeKind.Reinstall) return "Reinstall";
                return "Install";
            }
        }

        private void SetPrimary(string text)
        {
            primaryButton.Content = text;
            AutomationProperties.SetName(primaryButton, text);
        }

        private void Back()
        {
            if (current == Page.License) ShowPage(Page.Welcome);
            else if (current == Page.Options) ShowPage(Page.License);
        }

        private void Primary()
        {
            switch (current)
            {
                case Page.Welcome:
                    if (Uninstalling) StartUninstall();
                    else ShowPage(Page.License);
                    break;
                case Page.License: ShowPage(Page.Options); break;
                case Page.Options: StartInstall(); break;
                case Page.Finish: Finish(); break;
            }
        }

        private void OnAcceptChanged()
        {
            bool accepted = acceptCheck.IsChecked == true;
            if (current == Page.License) primaryButton.IsEnabled = accepted;
            acceptHelp.Visibility = accepted ? Visibility.Hidden : Visibility.Visible;
        }

        private void RefreshOptions()
        {
            if (pathBox == null || availableText == null) return;
            string destination = pathBox.Text.Trim();
            bool valid = InstallerPolicy.ValidDestination(destination);
            long? free = valid ? Engine.FreeBytes(destination) : null;
            availableLabel.Text = valid ? "Available on " + Engine.VolumeLabel(destination) : "Available space";
            availableText.Text = free.HasValue ? InstallerPolicy.FormatSize(free.Value) : "Unknown";

            string message = kind == UpgradeKind.Fresh ? "Installs for your account." : "Updates your existing installation.";
            string color = palette.Muted;
            if (!valid)
            {
                message = "Enter a full folder path, such as C:\\Users\\you\\Programs\\Forge Room. A drive root is not allowed.";
                color = palette.Text;
            }
            else if (!free.HasValue)
            {
                message = "Free space could not be read for this location.";
                color = palette.Text;
            }
            else if (free.Value < requiredBytes)
            {
                message = "Not enough free space here. Choose another location.";
                color = palette.Text;
            }
            pathMessage.Text = message;
            pathMessage.Foreground = Brush(color);

            if (current == Page.Options)
                primaryButton.IsEnabled = InstallerPolicy.CanInstall(acceptCheck.IsChecked == true, valid, free, requiredBytes);
        }

        private void Browse()
        {
            using (System.Windows.Forms.FolderBrowserDialog dialog = new System.Windows.Forms.FolderBrowserDialog())
            {
                dialog.Description = "Choose the folder for Forge Room";
                dialog.ShowNewFolderButton = true;
                if (Directory.Exists(pathBox.Text)) dialog.SelectedPath = pathBox.Text;
                if (dialog.ShowDialog() == System.Windows.Forms.DialogResult.OK)
                {
                    string chosen = dialog.SelectedPath;
                    // Keep the product folder when the user picks an existing parent.
                    pathBox.Text = Directory.Exists(chosen) && Directory.GetFileSystemEntries(chosen).Length > 0 &&
                                   !chosen.EndsWith("Forge Room Early Preview", StringComparison.OrdinalIgnoreCase)
                        ? System.IO.Path.Combine(chosen, "Forge Room Early Preview")
                        : chosen;
                }
            }
        }

        private void ShowNotes()
        {
            Window notes = new Window
            {
                Title = "Preview notes", Owner = this, Width = 640, Height = 480, WindowStartupLocation = WindowStartupLocation.CenterOwner,
                Background = Brush(palette.Bg), ShowInTaskbar = false
            };
            notes.Resources.MergedDictionaries.Add(Resources.MergedDictionaries[0]);
            TextBox box = new TextBox
            {
                Text = Embedded.Text("EARLY_PREVIEW_NOTES.md").Replace("\r\n", "\n"), IsReadOnly = true, TextWrapping = TextWrapping.Wrap,
                Background = Brush(palette.Field), Foreground = Brush(palette.Text), BorderThickness = new Thickness(0), FontSize = 14,
                Padding = new Thickness(18), VerticalScrollBarVisibility = ScrollBarVisibility.Auto
            };
            notes.Content = box;
            notes.KeyDown += delegate(object s, KeyEventArgs e) { if (e.Key == Key.Escape) notes.Close(); };
            notes.ShowDialog();
        }

        // ---- Install ---------------------------------------------------------------------

        private void StartInstall()
        {
            string destination = pathBox.Text.Trim().TrimEnd('\\');
            installedDirectory = destination;
            ShowPage(Page.Progress);
            installing = true;
            try
            {
                engineExe = Engine.Extract(out engineWork);
                logPath = System.IO.Path.Combine(engineWork, "install.log");
                ProcessStartInfo info = new ProcessStartInfo(engineExe, Engine.BuildArguments(logPath, destination, desktopCheck.IsChecked == true));
                info.UseShellExecute = false;
                info.CreateNoWindow = true;
                engineProcess = Process.Start(info);
            }
            catch (Exception ex)
            {
                installing = false;
                ShowResult(false, "Setup could not start the installer engine.\n" + ex.Message);
                return;
            }
            timer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(150) };
            timer.Tick += delegate { PollEngine(); };
            timer.Start();
        }

        private void PollEngine()
        {
            int files = 0, icons = 0;
            string tail = "";
            string problem = "";
            try
            {
                if (File.Exists(logPath))
                {
                    using (FileStream stream = new FileStream(logPath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete))
                    using (StreamReader reader = new StreamReader(stream))
                    {
                        string line;
                        while ((line = reader.ReadLine()) != null)
                        {
                            if (line.IndexOf("-- File entry --", StringComparison.Ordinal) >= 0) files++;
                            else if (line.IndexOf("-- Icon entry --", StringComparison.Ordinal) >= 0) icons++;
                            if (line.Length > 24)
                            {
                                string text = line.Substring(24);
                                tail = text;
                                if (text.IndexOf("error", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                    text.IndexOf("exception", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                    text.IndexOf("failed", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                    text.IndexOf("abort", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                    text.IndexOf("cancel", StringComparison.OrdinalIgnoreCase) >= 0 ||
                                    text.IndexOf("still running", StringComparison.OrdinalIgnoreCase) >= 0)
                                    problem = text.Trim();
                            }
                        }
                    }
                }
            }
            catch (IOException) { }

            double value = Math.Min(92, files * 15 + icons * 6);
            if (value > progress.Value) progress.Value = value;
            progressStatus.Text = files == 0 ? "Preparing\u2026" : "Installing files\u2026";

            if (engineProcess != null && engineProcess.HasExited)
            {
                timer.Stop();
                installing = false;
                int code = engineProcess.ExitCode;
                string log = problem.Length > 0 ? problem : tail;
                if (code == 0)
                {
                    progress.Value = 100;
                    Engine.RegisterUninstaller(installedDirectory);
                    ShowResult(true, (kind == UpgradeKind.Update ? "Updated to version " + meta["version"] + " in " : "Installed to ") + installedDirectory);
                }
                else
                {
                    ShowResult(false, "The installer engine stopped with code " + code + ".\n" + log + "\nNothing else was changed. Close setup and try again, or choose a different folder.");
                }
            }
        }

        private void StartUninstall()
        {
            ShowPage(Page.Progress);
            installing = true;
            progressStatus.Text = "Removing files\u2026";
            timer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(120) };
            timer.Tick += delegate { progress.Value += (92 - progress.Value) * 0.04; };
            timer.Start();
            string directory = uninstallDirectory;
            System.Threading.Thread worker = new System.Threading.Thread(delegate()
            {
                string error = Engine.Uninstall(directory);
                Dispatcher.BeginInvoke(new Action(delegate
                {
                    timer.Stop();
                    installing = false;
                    if (error == null)
                    {
                        progress.Value = 100;
                        ShowResult(true, "Forge Room was removed from " + directory + ". Your own files were not touched.");
                    }
                    else ShowResult(false, error);
                }));
            });
            worker.IsBackground = true;
            worker.Start();
        }

        // The uninstaller runs from a temporary copy; remove that copy once the window is gone.
        private static void RemoveOwnTempFolder()
        {
            try
            {
                string directory = System.IO.Path.GetDirectoryName(Assembly.GetExecutingAssembly().Location);
                if (!System.IO.Path.GetFileName(directory).StartsWith("ForgeRoomUninstall-", StringComparison.Ordinal)) return;
                ProcessStartInfo info = new ProcessStartInfo("cmd.exe", "/c ping -n 3 127.0.0.1 >nul & rd /s /q " + InstallerPolicy.Quote(directory));
                info.CreateNoWindow = true;
                info.UseShellExecute = false;
                Process.Start(info);
            }
            catch (Exception) { }
        }

        private void ShowResult(bool ok, string detail)
        {
            succeeded = ok;
            ShowPage(Page.Finish);
            finishHeading.Text = Uninstalling ? (ok ? "Forge Room was removed" : "Forge Room was not removed")
                : ok ? (kind == UpgradeKind.Update ? "Forge Room is updated" : "Forge Room is installed") : "Setup could not finish";
            finishSubtitle.Text = ok ? "Setup completed successfully." : (Uninstalling ? "Uninstall did not complete." : "Installation did not complete.");
            finishDetail.Text = detail;
            launchCheck.Visibility = (ok && !Uninstalling) ? Visibility.Visible : Visibility.Collapsed;
            SetPrimary(ok && !Uninstalling ? "Finish" : "Close");
        }

        private void Finish()
        {
            if (succeeded && launchCheck.IsChecked == true)
            {
                try
                {
                    string exe = System.IO.Path.Combine(installedDirectory, "forge-room-early-preview.exe");
                    if (File.Exists(exe)) Process.Start(new ProcessStartInfo(exe) { UseShellExecute = false, WorkingDirectory = installedDirectory });
                }
                catch (Exception) { }
            }
            Close();
        }

        private void OnClosing(object sender, System.ComponentModel.CancelEventArgs e)
        {
            if (installing) { e.Cancel = true; return; }
            if (timer != null) timer.Stop();
            Engine.Cleanup(engineWork);
            if (Uninstalling) RemoveOwnTempFolder();
        }
    }
}
