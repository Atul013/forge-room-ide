using System;
using System.Globalization;
using System.IO;
using System.Text;
using System.Text.RegularExpressions;

namespace ForgeRoom.Setup
{
    // Pure decision logic for the Ember setup host. No UI, no I/O beyond path parsing,
    // so it can be checked by PolicyTests without launching a window.
    public enum UpgradeKind { Fresh, Update, Reinstall, Downgrade }

    public static class InstallerPolicy
    {
        // Quote one command-line argument so it stays a single argument for the engine.
        public static string Quote(string value)
        {
            if (value == null) value = "";
            StringBuilder result = new StringBuilder("\"");
            int backslashes = 0;
            foreach (char c in value)
            {
                if (c == '\\')
                {
                    backslashes++;
                    continue;
                }
                if (c == '"')
                {
                    result.Append('\\', backslashes * 2 + 1);
                    result.Append('"');
                }
                else
                {
                    result.Append('\\', backslashes);
                    result.Append(c);
                }
                backslashes = 0;
            }
            // Backslashes before the closing quote must be doubled or they escape it.
            result.Append('\\', backslashes * 2);
            result.Append('"');
            return result.ToString();
        }

        // Parse a size such as "At least 22.5 MB of free disk space is required." into bytes.
        // Returns null when no size is present; an unknown requirement must never read as zero.
        public static long? RequiredBytes(string message)
        {
            if (message == null) return null;
            Match match = Regex.Match(message, @"([0-9]+(?:[.,][0-9]+)?)\s*(KB|MB|GB)", RegexOptions.IgnoreCase);
            if (!match.Success) return null;
            double amount;
            string number = match.Groups[1].Value.Replace(',', '.');
            if (!double.TryParse(number, NumberStyles.Float, CultureInfo.InvariantCulture, out amount)) return null;
            double unit;
            switch (match.Groups[2].Value.ToUpperInvariant())
            {
                case "KB": unit = 1024.0; break;
                case "MB": unit = 1024.0 * 1024.0; break;
                default: unit = 1024.0 * 1024.0 * 1024.0; break;
            }
            return (long)Math.Round(amount * unit);
        }

        // An install destination must be an absolute local or network folder, never a drive root.
        public static bool ValidDestination(string path)
        {
            if (string.IsNullOrWhiteSpace(path)) return false;
            string trimmed = path.Trim();
            bool drive = trimmed.Length >= 3 && char.IsLetter(trimmed[0]) && trimmed[1] == ':' &&
                         (trimmed[2] == '\\' || trimmed[2] == '/');
            bool unc = trimmed.StartsWith("\\\\", StringComparison.Ordinal) && trimmed.Length > 2 && trimmed[2] != '\\';
            if (!drive && !unc) return false;

            string rest = drive ? trimmed.Substring(2) : trimmed.Substring(2);
            if (rest.IndexOfAny(new char[] { '<', '>', ':', '"', '|', '?', '*' }) >= 0) return false;
            foreach (char c in trimmed)
            {
                if (c < ' ') return false;
            }
            try
            {
                string full = Path.GetFullPath(trimmed);
                string root = Path.GetPathRoot(full);
                if (string.IsNullOrEmpty(root)) return false;
                if (string.Equals(full.TrimEnd('\\'), root.TrimEnd('\\'), StringComparison.OrdinalIgnoreCase)) return false;
                if (unc)
                {
                    // \\server\share\folder needs at least a folder below the share.
                    string[] parts = full.TrimStart('\\').Split(new char[] { '\\' }, StringSplitOptions.RemoveEmptyEntries);
                    if (parts.Length < 3) return false;
                }
            }
            catch (ArgumentException) { return false; }
            catch (NotSupportedException) { return false; }
            catch (PathTooLongException) { return false; }
            return true;
        }

        // Install is only possible with consent, a valid folder and verified free space.
        public static bool CanInstall(bool accepted, bool validDestination, long? freeBytes, long? requiredBytes)
        {
            if (!accepted || !validDestination) return false;
            if (!freeBytes.HasValue || !requiredBytes.HasValue) return false;
            return freeBytes.Value >= requiredBytes.Value;
        }

        // Semantic-version style comparison: 1.10.0 > 1.9.0, and 1.0.0-alpha.1 < 1.0.0.
        // Returns <0, 0 or >0. Unparseable input compares equal so it never blocks an install.
        public static int CompareVersions(string left, string right)
        {
            int[] a, b;
            string preA, preB;
            if (!TryParseVersion(left, out a, out preA) || !TryParseVersion(right, out b, out preB)) return 0;
            for (int i = 0; i < Math.Max(a.Length, b.Length); i++)
            {
                int x = i < a.Length ? a[i] : 0;
                int y = i < b.Length ? b[i] : 0;
                if (x != y) return x < y ? -1 : 1;
            }
            if (preA == preB) return 0;
            if (preA.Length == 0) return 1;   // a release outranks any pre-release of the same core
            if (preB.Length == 0) return -1;
            string[] ids = preA.Split('.');
            string[] other = preB.Split('.');
            for (int i = 0; i < Math.Min(ids.Length, other.Length); i++)
            {
                int n1, n2;
                bool num1 = int.TryParse(ids[i], out n1);
                bool num2 = int.TryParse(other[i], out n2);
                int cmp;
                if (num1 && num2) cmp = n1.CompareTo(n2);
                else if (num1) cmp = -1;
                else if (num2) cmp = 1;
                else cmp = string.CompareOrdinal(ids[i], other[i]);
                if (cmp != 0) return cmp < 0 ? -1 : 1;
            }
            return ids.Length.CompareTo(other.Length);
        }

        private static bool TryParseVersion(string text, out int[] parts, out string prerelease)
        {
            parts = null;
            prerelease = "";
            if (string.IsNullOrWhiteSpace(text)) return false;
            string core = text.Trim();
            int dash = core.IndexOf('-');
            if (dash >= 0)
            {
                prerelease = core.Substring(dash + 1);
                core = core.Substring(0, dash);
            }
            string[] pieces = core.Split('.');
            parts = new int[pieces.Length];
            for (int i = 0; i < pieces.Length; i++)
            {
                if (!int.TryParse(pieces[i], out parts[i])) return false;
            }
            return true;
        }

        // What running this setup means for the machine it runs on.
        public static UpgradeKind Classify(string installedVersion, string newVersion)
        {
            if (string.IsNullOrWhiteSpace(installedVersion)) return UpgradeKind.Fresh;
            int cmp = CompareVersions(installedVersion, newVersion);
            if (cmp < 0) return UpgradeKind.Update;
            if (cmp == 0) return UpgradeKind.Reinstall;
            return UpgradeKind.Downgrade;
        }

        public static string FormatSize(long bytes)
        {
            // Decimal units, matching the approved mockup (14,360,419 bytes shows as 14.4 MB).
            const double MB = 1000.0 * 1000.0;
            const double GB = MB * 1000.0;
            if (bytes >= GB) return (bytes / GB).ToString("0.0", CultureInfo.InvariantCulture) + " GB";
            return (bytes / MB).ToString("0.0", CultureInfo.InvariantCulture) + " MB";
        }
    }
}
