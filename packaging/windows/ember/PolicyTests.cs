using System;
using ForgeRoom.Setup;

internal static class PolicyTests
{
    private static void Assert(bool value, string message)
    {
        if (!value) throw new Exception(message);
    }

    public static int Main()
    {
        Assert(InstallerPolicy.Quote("C:\\My Files\\") == "\"C:\\My Files\\\\\"", "Trailing slash must not escape the closing quote");
        Assert(InstallerPolicy.Quote("a\"b") == "\"a\\\"b\"", "An embedded quote must stay inside one argument");
        Assert(InstallerPolicy.Quote("") == "\"\"", "An empty task list must remain a real argument");
        Assert(InstallerPolicy.RequiredBytes("At least 22.5 MB of free disk space is required.") == 23592960, "Use the backend's real requirement");
        Assert(InstallerPolicy.RequiredBytes("Missing size") == null, "Unknown requirements must not become zero");
        Assert(!InstallerPolicy.ValidDestination("relative\\folder"), "Reject relative destinations");
        Assert(!InstallerPolicy.ValidDestination("C:\\"), "Do not install into a drive root");
        Assert(!InstallerPolicy.ValidDestination("C:\\bad?\\Forge Room"), "Reject invalid path characters");
        Assert(InstallerPolicy.ValidDestination("E:\\Project Files\\Forge Room"), "Accept absolute paths with spaces");
        Assert(InstallerPolicy.ValidDestination("\\\\server\\share\\Forge Room"), "Accept a valid network destination");
        Assert(!InstallerPolicy.CanInstall(false, true, 100, 1), "Agreement is mandatory");
        Assert(!InstallerPolicy.CanInstall(true, true, 1, 100), "Insufficient disk space must block Install");
        Assert(!InstallerPolicy.CanInstall(true, true, null, 100), "Unknown free space must block Install");
        Assert(InstallerPolicy.CanInstall(true, true, 100, 100), "A valid destination with enough space is allowed");
        Assert(InstallerPolicy.CompareVersions("1.3.0", "1.1.0") > 0, "1.3.0 is newer than 1.1.0");
        Assert(InstallerPolicy.CompareVersions("1.1", "1.3") < 0, "Short versions compare numerically");
        Assert(InstallerPolicy.CompareVersions("1.10.0", "1.9.0") > 0, "Compare numbers, not text");
        Assert(InstallerPolicy.CompareVersions("1.1.0", "1.1") == 0, "Missing parts count as zero");
        Assert(InstallerPolicy.CompareVersions("1.0.0-alpha.1", "1.0.0") < 0, "A pre-release is older than its release");
        Assert(InstallerPolicy.CompareVersions("0.109.0-alpha.2", "0.109.0-alpha.1") > 0, "Later pre-release is newer");
        Assert(InstallerPolicy.Classify(null, "1.3.0") == UpgradeKind.Fresh, "No installed version means a fresh install");
        Assert(InstallerPolicy.Classify("1.1.0", "1.3.0") == UpgradeKind.Update, "Older installed version is updated");
        Assert(InstallerPolicy.Classify("1.3.0", "1.3.0") == UpgradeKind.Reinstall, "Same version is a reinstall");
        Assert(InstallerPolicy.Classify("1.3.0", "1.1.0") == UpgradeKind.Downgrade, "Newer installed version must not be downgraded");
        Console.WriteLine("24 installer policy checks passed.");
        return 0;
    }
}
