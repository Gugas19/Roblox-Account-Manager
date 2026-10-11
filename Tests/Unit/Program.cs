using RBX_Alt_Manager;
using RBX_Alt_Manager.Classes;
using System;
using System.Collections;
using System.Collections.Generic;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;
using System.Text.RegularExpressions;
using System.Globalization;
using System.Threading;
using System.Web.Script.Serialization;

namespace RAM.UnitTests
{
    /// <summary>
    /// Unit tests for Roblox Account Manager, run against the built RobloxAccountManager.exe.
    /// Exit code 0 when everything passed. Usage: RAM.UnitTests.exe [repository root]
    /// </summary>
    internal static class Program
    {
        private static int Passed, Failed;
        private static readonly List<string> Failures = new List<string>();
        private static string RepoRoot, WorkFolder;

        private static readonly Assembly App = typeof(Lang).Assembly;
        private static readonly string[] Translated = { "pt-PT", "es", "fr", "de" };

        [STAThread]
        private static int Main(string[] Args)
        {
            Console.OutputEncoding = Encoding.UTF8;
            RepoRoot = Args.Length > 0 ? Args[0] : FindRepoRoot();

            // the app saves next to its working folder, keep everything the tests write in a temporary one
            WorkFolder = Path.Combine(Path.GetTempPath(), "RAM.UnitTests." + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(WorkFolder);
            Environment.CurrentDirectory = WorkFolder;

            Console.WriteLine($"Roblox Account Manager unit tests\nApp: {App.Location}\nRepository: {RepoRoot}\n");

            TranslationTests();
            SourceStringTests();
            ThemeTests();
            EncryptionTests();
            IniTests();
            ModernUITests();
            LogicTests();

            try { Environment.CurrentDirectory = Path.GetTempPath(); Directory.Delete(WorkFolder, true); } catch { }

            Console.WriteLine($"\n{Passed} passed, {Failed} failed");

            foreach (string Failure in Failures)
                Console.WriteLine("  FAILED " + Failure);

            return Failed == 0 ? 0 : 1;
        }

        #region Mini framework

        private static void Test(string Name, Action Body)
        {
            try
            {
                Body();
                Passed++;
                Console.WriteLine("  ok    " + Name);
            }
            catch (Exception x)
            {
                Failed++;
                string Message = (x is TargetInvocationException ? x.InnerException : x)?.Message;
                Failures.Add($"{Name}: {Message}");
                Console.WriteLine($"  FAIL  {Name}: {Message}");
            }
        }

        private static void Assert(bool Condition, string Message)
        {
            if (!Condition) throw new Exception(Message);
        }

        private static void Equal<T>(T Expected, T Actual, string What) =>
            Assert(EqualityComparer<T>.Default.Equals(Expected, Actual), $"{What}: expected [{Expected}] but got [{Actual}]");

        private static string FindRepoRoot()
        {
            var Directory = new DirectoryInfo(AppDomain.CurrentDomain.BaseDirectory);

            while (Directory != null && !System.IO.Directory.Exists(Path.Combine(Directory.FullName, "RBX Alt Manager")))
                Directory = Directory.Parent;

            return Directory?.FullName ?? throw new Exception("Repository root not found, pass it as the first argument");
        }

        private static Dictionary<string, string> LanguageTable(string Code)
        {
            string Resource = App.GetManifestResourceNames().Single(Name => Name.EndsWith($".Lang.{Code}.json", StringComparison.OrdinalIgnoreCase));

            using (var Reader = new StreamReader(App.GetManifestResourceStream(Resource)))
                return new JavaScriptSerializer().Deserialize<Dictionary<string, string>>(Reader.ReadToEnd());
        }

        private static List<string> Placeholders(string Text) => Regex.Matches(Text, @"\{\d+\}").Cast<Match>().Select(M => M.Value).OrderBy(P => P).ToList();

        #endregion

        private static void TranslationTests()
        {
            Console.WriteLine("Translations");

            Test("5 languages with an embedded flag each", () =>
            {
                Equal(5, Lang.Languages.Length, "language count");

                foreach (Language language in Lang.Languages)
                    Assert(ModernUI.Flag(language.Flag, 14) != null, $"no flag for {language.Code}");
            });

            Test("every translated language has the same keys", () =>
            {
                var Reference = LanguageTable("pt-PT");

                foreach (string Code in Translated)
                {
                    var Table = LanguageTable(Code);
                    var Missing = Reference.Keys.Except(Table.Keys).ToList();
                    var Extra = Table.Keys.Except(Reference.Keys).ToList();

                    Assert(Missing.Count == 0 && Extra.Count == 0, $"{Code}: missing [{string.Join(" | ", Missing.Take(5))}] extra [{string.Join(" | ", Extra.Take(5))}]");
                }
            });

            Test("placeholders like {0} are kept in every translation", () =>
            {
                foreach (string Code in Translated)
                    foreach (var Entry in LanguageTable(Code))
                        Assert(Placeholders(Entry.Key).SequenceEqual(Placeholders(Entry.Value)), $"{Code}: \"{Entry.Key}\" -> \"{Entry.Value}\"");
            });

            Test("no empty translations", () =>
            {
                foreach (string Code in Translated)
                    foreach (var Entry in LanguageTable(Code))
                        Assert(!string.IsNullOrWhiteSpace(Entry.Value) || string.IsNullOrWhiteSpace(Entry.Key), $"{Code}: \"{Entry.Key}\" is empty");
            });

            Test("simple text is translated", () =>
            {
                Lang.SetLanguage("pt-PT", false);
                Equal("Adicionar conta", Lang.T("Add Account"), "pt-PT");
                Lang.SetLanguage("de", false);
                Equal("Konto hinzufügen", Lang.T("Add Account"), "de");
            });

            Test("text built from a template is translated", () =>
            {
                Lang.SetLanguage("pt-PT", false);
                Equal("Tem a certeza de que quer remover 3 contas?", Lang.T("Are you sure you want to remove 3 accounts?"), "template");
                Equal("2 de 6 contas", Lang.F("{0} of {1} accounts", 2, 6), "format");
            });

            Test("values inside a template are translated too", () =>
            {
                Lang.SetLanguage("pt-PT", false);
                Equal("Palavra-passe incorreta!\n\nPalavra-passe inválida, tem de ter pelo menos 4 caracteres",
                    Lang.T("Incorrect Password!\n\nInvalid password, your password must contain 4 or more characters"), "nested");
            });

            Test("unknown text and English stay unchanged", () =>
            {
                Lang.SetLanguage("pt-PT", false);
                Equal("NightFalcon_07", Lang.T("NightFalcon_07"), "username");
                Equal(null, Lang.T(null), "null");
                Lang.SetLanguage("en", false);
                Equal("Add Account", Lang.T("Add Account"), "English");
                Equal("Import Accounts", Lang.T("ImportForm"), "English window title fix");
            });

            Test("switching language raises LanguageChanged", () =>
            {
                int Raised = 0;
                EventHandler Handler = (s, e) => Raised++;

                Lang.LanguageChanged += Handler;
                Lang.SetLanguage("fr", false);
                Lang.SetLanguage("es", false);
                Lang.LanguageChanged -= Handler;

                Equal(2, Raised, "events");
                Equal("es", Lang.Current.Code, "current language");
            });

            Lang.SetLanguage("en", false);
        }

        /// <summary>Every text found in the source code has to be in every language file</summary>
        private static void SourceStringTests()
        {
            Console.WriteLine("Source code texts");

            Test("every UI text in the source code is translated", () =>
            {
                var Strings = ExtractSourceStrings(Path.Combine(RepoRoot, "RBX Alt Manager"));
                Assert(Strings.Count > 300, $"only {Strings.Count} texts found, the scanner is broken");

                foreach (string Code in Translated)
                {
                    var Table = LanguageTable(Code);
                    var Missing = Strings.Where(S => !Table.ContainsKey(S)).ToList();

                    Assert(Missing.Count == 0, $"{Code} misses {Missing.Count}: {string.Join(" | ", Missing.Take(8).Select(M => M.Replace("\n", "\\n")))}");
                }
            });
        }

        private static readonly HashSet<string> NotTranslated = new HashSet<string>
        {
            "Roblox Account Manager", "Account Manager", "RAMSettings.ini", "Chromium", "CefSharp", "Manual", "URL", "Javascript", "Roblox", "MB", "FPS",
            "isTeleport", "/UC", "title", "<Name>", "<Price>", "ToolTip", "Section", "Device: ", "Failed to load favorite games!\n", "Failed to purchase {0}: "
        };

        private static List<string> ExtractSourceStrings(string Folder)
        {
            const string Literal = @"(\$?)""((?:[^""\\]|\\.)*)""";
            var Result = new HashSet<string>();

            void Add(string Text)
            {
                if (string.IsNullOrEmpty(Text) || !Regex.IsMatch(Text, "[A-Za-z]{2}") || NotTranslated.Contains(Text) || Text.StartsWith("http") || ")}]".Contains(Text[0])) return;
                if (Regex.IsMatch(Text, @"^[A-Za-z_]+[A-Z][A-Za-z_]*$") && !Text.Contains(" ") && Text != Text.ToUpper() && Text.Any(char.IsLower) && Text.Skip(1).Any(char.IsUpper)) return; // identifiers like "WhoFollowLabel"

                Result.Add(Text);
            }

            foreach (string File in Directory.GetFiles(Folder, "*.cs", SearchOption.AllDirectories).Where(F => !F.Contains(@"\bin\") && !F.Contains(@"\obj\")))
            {
                string Text = System.IO.File.ReadAllText(File);

                if (File.EndsWith(".Designer.cs"))
                {
                    Text = Regex.Replace(Text, @"""\s*\+\s*\r?\n\s*""", "");

                    foreach (Match M in Regex.Matches(Text, @"\.(?:Text|ToolTipText|HeaderText) = ""((?:[^""\\]|\\.)*)"";")) Add(Unescape(M.Groups[1].Value));
                    foreach (Match M in Regex.Matches(Text, @"SetToolTip\([^,]+, ""((?:[^""\\]|\\.)*)""\)")) Add(Unescape(M.Groups[1].Value));
                    foreach (Match M in Regex.Matches(Text, @"Items\.AddRange\(new object\[\] \{(.*?)\}\);", RegexOptions.Singleline))
                        foreach (Match L in Regex.Matches(M.Groups[1].Value, Literal)) Add(Unescape(L.Groups[2].Value));

                    continue;
                }

                foreach (string Line in Text.Split('\n'))
                {
                    if (Line.Contains("Reply(") || Line.Contains("Logger.") || Line.Contains("WebServerResponse") || Line.TrimStart().StartsWith("//")) continue;

                    if (Regex.IsMatch(Line, @"MessageBox\.Show|YesNoPrompt|ShowDialog\(|Lang\.[TF]\(|SetPlaceholder|SetToolTip|SectionLabel\(|(?<![.\w])Section\(""|TaskDialogButton|return \$?""ERROR"))
                        foreach (Match L in Regex.Matches(Line, Literal))
                            Add(L.Groups[1].Value == "$" ? Template(Unescape(L.Groups[2].Value)) : Unescape(L.Groups[2].Value));
                    else if (Regex.Match(Line, @"\bText = ""((?:[^""\\]|\\.)*)""") is Match TextMatch && TextMatch.Success)
                        Add(Unescape(TextMatch.Groups[1].Value));
                }
            }

            return Result.ToList();
        }

        private static string Unescape(string Text) => Regex.Unescape(Text.Replace("\\'", "'"));

        /// <summary>$"Remove {count} accounts?" -> "Remove {0} accounts?"</summary>
        private static string Template(string Text)
        {
            var Builder = new StringBuilder();
            int Depth = 0, Index = 0;

            for (int i = 0; i < Text.Length; i++)
            {
                char C = Text[i];

                if (Depth == 0 && i + 1 < Text.Length && ((C == '{' && Text[i + 1] == '{') || (C == '}' && Text[i + 1] == '}'))) { Builder.Append(C); i++; continue; }
                if (C == '{') { if (Depth++ == 0) Builder.Append('{').Append(Index++).Append('}'); continue; }
                if (C == '}') { Depth--; continue; }
                if (Depth == 0) Builder.Append(C);
            }

            return Builder.ToString();
        }

        private static void ThemeTests()
        {
            Console.WriteLine("Theme");

            MethodInfo IsOldDefaultTheme = App.GetType("RBX_Alt_Manager.Program").GetMethod("IsOldDefaultTheme", BindingFlags.NonPublic | BindingFlags.Static);
            const string Old = "[RobloxAccountManager]\r\nAccountsBG=#1E1F28\r\nAccountsFG=#FFFFFF\r\nButtonsBG=#292929\r\nButtonsFG=#FFFFFF\r\nButtonsBC=#282828\r\nButtonStyle=Popup\r\nFormsBG=#292929\r\nFormsFG=#FFFFFF\r\nDarkTopBar=True\r\nLightImages=True\r\nLabelsTransparent=True\r\nShowHeaders=True\r\nTextBoxesBG=#3A3A3A\r\nTextBoxesFG=#FFFFFF\r\nTextBoxesBC=#282828";

            Test("the untouched old default theme is upgraded", () => Assert((bool)IsOldDefaultTheme.Invoke(null, new object[] { Old }), "not detected"));
            Test("a customized theme is kept", () => Assert(!(bool)IsOldDefaultTheme.Invoke(null, new object[] { Old.Replace("#1E1F28", "#FF0000") }), "custom theme would be replaced"));
            Test("the new default theme isn't seen as the old one", () =>
            {
                string NewDefault = (string)App.GetType("RBX_Alt_Manager.Properties.Resources").GetProperty("DefaultTheme", BindingFlags.NonPublic | BindingFlags.Static).GetValue(null);

                Assert(NewDefault.Contains("AccentColor"), "new default has no accent color");
                Assert(!(bool)IsOldDefaultTheme.Invoke(null, new object[] { NewDefault }), "new default detected as old");
            });
        }

        private static void EncryptionTests()
        {
            Console.WriteLine("Encryption");

            Type Cryptography = App.GetType("RBX_Alt_Manager.Classes.Cryptography");
            MethodInfo Encrypt = Cryptography.GetMethod("Encrypt"), Decrypt = Cryptography.GetMethod("Decrypt");
            byte[] Key = Encoding.UTF8.GetBytes("unit-test-password-hash-32bytes!");

            Test("encrypted account data decrypts with the right password", () =>
            {
                const string Data = "[{\"Username\":\"Test\",\"SecurityToken\":\"abc\"}]";
                byte[] Encrypted = (byte[])Encrypt.Invoke(null, new object[] { Data, Key });

                Assert(!Encoding.UTF8.GetString(Encrypted).Contains("abc"), "plain text found in the encrypted data");
                Equal(Data, Encoding.UTF8.GetString((byte[])Decrypt.Invoke(null, new object[] { Encrypted, Key })), "round trip");
            });

            Test("a wrong password can't decrypt", () =>
            {
                byte[] Encrypted = (byte[])Encrypt.Invoke(null, new object[] { "secret", Key });
                bool Threw = false;

                try { Decrypt.Invoke(null, new object[] { Encrypted, Encoding.UTF8.GetBytes("wrong-password-hash-of-32-bytes!") }); }
                catch (TargetInvocationException) { Threw = true; }

                Assert(Threw, "decrypted with the wrong password");
            });
        }

        private static void IniTests()
        {
            Console.WriteLine("Settings file");

            Test("settings are read and written back", () =>
            {
                var Ini = new IniFile(new StringReader("[General]\nLanguage=pt-PT\n# comment\nEnableMultiRbx=true\n\n[WebServer]\nPassword=abc=def\n"));

                Equal("pt-PT", Ini.Section("General").Get("Language"), "Language");
                Equal(true, Ini.Section("General").Get<bool>("EnableMultiRbx"), "EnableMultiRbx");
                Equal("abc=def", Ini.Section("WebServer").Get("Password"), "value containing =");

                Ini.Section("General").Set("Language", "de");

                var Writer = new StringWriter();
                Ini.Save(Writer);
                var Again = new IniFile(new StringReader(Writer.ToString()));

                Equal("de", Again.Section("General").Get("Language"), "after saving");
            });

            Test("setting an empty value removes it", () =>
            {
                var Ini = new IniFile(new StringReader("[General]\nA=1\n"));
                Ini.Section("General").Set("A", "");

                Assert(!Ini.Section("General").Exists("A"), "still exists");
            });
        }

        private static void ModernUITests()
        {
            Console.WriteLine("Look");

            Test("color helpers", () =>
            {
                Equal(Color.FromArgb(255, 127, 127, 127).ToArgb(), ModernUI.Mix(Color.Black, Color.White, 0.5f).ToArgb(), "mix");
                Assert(ModernUI.Shift(Color.Black, 0.2f).GetBrightness() > 0, "dark colors get lighter");
                Assert(ModernUI.Shift(Color.White, 0.2f).GetBrightness() < 1, "light colors get darker");
                Equal(Color.White.ToArgb(), ModernUI.ContrastText(Color.FromArgb(30, 30, 30)).ToArgb(), "text on dark");
                Assert(ModernUI.ContrastText(Color.White).GetBrightness() < 0.2f, "text on light");
            });

            Test("every icon used by a button or menu is embedded", () =>
            {
                var Names = new HashSet<string>();

                foreach (string Field in new[] { "ButtonIcons", "MenuIcons" })
                    foreach (string Icon in ((IDictionary)typeof(ModernUI).GetField(Field, BindingFlags.NonPublic | BindingFlags.Static).GetValue(null)).Values)
                        Names.Add(Icon);

                foreach (IDictionary Form in ((IDictionary)typeof(ModernUI).GetField("FormButtonIcons", BindingFlags.NonPublic | BindingFlags.Static).GetValue(null)).Values)
                    foreach (string Icon in Form.Values)
                        if (Icon != null) Names.Add(Icon);

                Names.UnionWith(new[] { "search", "history", "shuffle" });

                var Missing = Names.Where(Name => ModernUI.Icon(Name, 16, Color.White) == null).ToList();
                Assert(Missing.Count == 0, "missing icons: " + string.Join(", ", Missing));
            });

            Test("icons are recolored and sized", () =>
            {
                Bitmap Icon = ModernUI.Icon("settings", 20, Color.FromArgb(255, 10, 200, 30));
                Equal(20, Icon.Width, "width");

                bool Found = false;

                for (int x = 0; x < Icon.Width && !Found; x++)
                    for (int y = 0; y < Icon.Height && !Found; y++)
                    {
                        Color Pixel = Icon.GetPixel(x, y);
                        Found = Pixel.A > 200 && Pixel.R == 10 && Pixel.G == 200 && Pixel.B == 30;
                    }

                Assert(Found, "no pixel in the requested color");
            });

            Test("the browser is kept in the app's own folder", () =>
            {
                string Folder = (string)App.GetType("RBX_Alt_Manager.Classes.AccountBrowser").GetField("BrowserFolder").GetValue(null);

                Assert(Folder.StartsWith(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData)) && Folder.Contains("RobloxAccountManager"), Folder);
            });
        }

        /// <summary>Behaviour of the app's own code, run in this process: no window is opened or clicked</summary>
        private static void LogicTests()
        {
            Console.WriteLine("Logic");

            FieldInfo General = typeof(AccountManager).GetField("General");
            FieldInfo StartTime = typeof(AccountManager).GetField("startTime", BindingFlags.NonPublic | BindingFlags.Static);
            string SaveFile = Path.Combine(WorkFolder, "AccountData.json");

            void UseSettings(string Ini) => General.SetValue(null, new IniFile(new StringReader(Ini)).Section("General"));

            Test("a saved pt-BR language falls back to pt-PT", () =>
            {
                UseSettings("[General]\nLanguage=pt-BR\n");
                Lang.Initialize();
                Equal("pt-PT", Lang.Current.Code, "language");
            });

            Test("Portuguese systems start in pt-PT, others in their language or English", () =>
            {
                CultureInfo Before = Thread.CurrentThread.CurrentUICulture;
                UseSettings("[General]\n");

                try
                {
                    foreach (var (Culture, Expected) in new[] { ("pt-BR", "pt-PT"), ("pt-PT", "pt-PT"), ("de-AT", "de"), ("ja-JP", "en") })
                    {
                        Thread.CurrentThread.CurrentUICulture = new CultureInfo(Culture);
                        Lang.Initialize();
                        Equal(Expected, Lang.Current.Code, Culture);
                    }
                }
                finally { Thread.CurrentThread.CurrentUICulture = Before; Lang.SetLanguage("en", false); }
            });

            // the accounts are saved as plain JSON when this file exists, no password needed for the tests
            File.WriteAllText(Path.Combine(WorkFolder, "NoEncryption.IUnderstandTheRisks.iautamor"), "");
            UseSettings("[General]\n");

            Test("accounts aren't saved in the first 2 seconds after starting", () =>
            {
                File.Delete(SaveFile);
                StartTime.SetValue(null, DateTime.Now);
                AccountManager.AccountsList = new List<Account> { new Account { Username = "UnitTest", UserID = 1 } };
                AccountManager.SaveAccounts();

                Assert(!File.Exists(SaveFile), "saved right after starting");
            });

            Test("accounts are saved at any second of a minute (it used to skip 2 s of every minute)", () =>
            {
                foreach (double Seconds in new[] { 60.5, 121.2, 75 })
                {
                    File.Delete(SaveFile);
                    StartTime.SetValue(null, DateTime.Now.AddSeconds(-Seconds));
                    AccountManager.SaveAccounts();

                    Assert(File.Exists(SaveFile) && File.ReadAllText(SaveFile).Contains("UnitTest"), $"not saved {Seconds} s after starting");
                }
            });

            Test("alias and description changes are saved, too long values are refused", () =>
            {
                Account Account = AccountManager.AccountsList[0];

                Account.Alias = "Main alt";
                Account.Description = "Farm account";
                Assert(File.ReadAllText(SaveFile).Contains("Main alt") && File.ReadAllText(SaveFile).Contains("Farm account"), "not saved");

                Account.Alias = new string('a', 51);
                Equal("Main alt", Account.Alias, "alias over 50 characters");
                Account.Description = new string('a', 5001);
                Equal("Farm account", Account.Description, "description over 5000 characters");
            });

            Test("an empty account list never overwrites the saved accounts", () =>
            {
                string Saved = File.ReadAllText(SaveFile);

                AccountManager.AccountsList = new List<Account>();
                AccountManager.SaveAccounts();

                Equal(Saved, File.ReadAllText(SaveFile), "saved accounts");
            });

            Test("saving keeps a backup of the previous file", () =>
            {
                AccountManager.AccountsList = new List<Account> { new Account { Username = "UnitTest2", UserID = 2 } };
                File.Delete(SaveFile + ".backup");
                AccountManager.SaveAccounts();

                Assert(File.Exists(SaveFile + ".backup") && File.ReadAllText(SaveFile + ".backup").Contains("UnitTest"), "no backup of the old accounts");
                Assert(File.ReadAllText(SaveFile).Contains("UnitTest2"), "new accounts not saved");
            });

            AccountManager.AccountsList = null;

            Test("a web server that can't start frees its port, so the local-only retry works", () =>
            {
                int Port = new Random().Next(20000, 40000);
                bool FirstFailed = false;

                try { new WebServer(Context => "", $"http://localhost:{Port}/", $"http://*:{Port}/").Stop(); } // every address needs administrator rights
                catch (System.Net.HttpListenerException) { FirstFailed = true; }

                if (!FirstFailed) return; // running as administrator, nothing to check

                var Local = new WebServer(Context => "", $"http://localhost:{Port}/"); // used to fail with "access denied"
                Local.Stop();
            });

            Test("every message box is translated", () =>
            {
                var Untranslated = Directory.GetFiles(Path.Combine(RepoRoot, "RBX Alt Manager"), "*.cs", SearchOption.AllDirectories)
                    .Where(F => !F.Contains(@"\obj\") && !F.EndsWith("Lang.cs") && !F.EndsWith("Program.cs"))
                    .Where(F => File.ReadAllText(F).Contains("MessageBox.Show(") && !File.ReadAllText(F).Contains("using MessageBox = RBX_Alt_Manager.Classes.LocalizedMessageBox;"))
                    .Select(Path.GetFileName).ToList();

                Assert(Untranslated.Count == 0, "message boxes not translated in " + string.Join(", ", Untranslated));
            });

            Test("check boxes show their tick on a dark theme", () =>
            {
                RBX_Alt_Manager.Forms.ThemeEditor.FormsBackground = Color.FromArgb(24, 26, 34);
                RBX_Alt_Manager.Forms.ThemeEditor.FormsForeground = Color.White;
                RBX_Alt_Manager.Forms.ThemeEditor.TextBoxesBackground = Color.FromArgb(34, 36, 46);
                RBX_Alt_Manager.Forms.ThemeEditor.TextBoxesForeground = Color.White;

                using (var Form = new System.Windows.Forms.Form { BackColor = Color.FromArgb(24, 26, 34), ForeColor = Color.White })
                using (var Check = new System.Windows.Forms.CheckBox { Text = "Enable Developer Mode", AutoSize = true, BackColor = Color.Transparent, ForeColor = Color.White })
                {
                    Form.Controls.Add(Check);
                    ModernUI.Apply(Form);

                    Color BoxCenter(bool Checked)
                    {
                        Check.Checked = Checked;
                        using (var Picture = new Bitmap(Check.Width, Check.Height))
                        {
                            Check.DrawToBitmap(Picture, new Rectangle(0, 0, Check.Width, Check.Height));
                            int Size = (int)Math.Round(13 * Check.DeviceDpi / 96f);
                            return Picture.GetPixel((int)(Size * 0.3f), (Check.Height - Size) / 2 + (int)(Size * 0.25f)); // inside the box, above the tick
                        }
                    }

                    Color Off = BoxCenter(false), On = BoxCenter(true);

                    Assert(Off.GetBrightness() < 0.4f, $"unchecked box is bright ({Off}), the white tick wouldn't be visible");
                    Assert(Math.Abs(On.B - ModernUI.Accent.B) < 30 && Math.Abs(On.R - ModernUI.Accent.R) < 30, $"checked box isn't accent colored ({On})");
                }
            });
        }
    }
}
