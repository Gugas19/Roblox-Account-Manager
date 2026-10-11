using BrightIdeasSoftware;
using Newtonsoft.Json;
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Text.RegularExpressions;
using System.Windows.Forms;

namespace RBX_Alt_Manager.Classes
{
    public class Language
    {
        public string Code { get; }
        public string Name { get; }
        public string Flag { get; }

        public Language(string Code, string Name, string Flag)
        {
            this.Code = Code;
            this.Name = Name;
            this.Flag = Flag;
        }
    }

    /// <summary>MessageBox that translates its texts, used through "using MessageBox = ..." aliases</summary>
    public static class LocalizedMessageBox
    {
        public static DialogResult Show(string Text) => System.Windows.Forms.MessageBox.Show(Lang.T(Text));
        public static DialogResult Show(string Text, string Caption) => System.Windows.Forms.MessageBox.Show(Lang.T(Text), Lang.T(Caption));
        public static DialogResult Show(string Text, string Caption, MessageBoxButtons Buttons) => System.Windows.Forms.MessageBox.Show(Lang.T(Text), Lang.T(Caption), Buttons);
        public static DialogResult Show(string Text, string Caption, MessageBoxButtons Buttons, MessageBoxIcon Icon) => System.Windows.Forms.MessageBox.Show(Lang.T(Text), Lang.T(Caption), Buttons, Icon);
        public static DialogResult Show(string Text, string Caption, MessageBoxButtons Buttons, MessageBoxIcon Icon, MessageBoxDefaultButton DefaultButton) => System.Windows.Forms.MessageBox.Show(Lang.T(Text), Lang.T(Caption), Buttons, Icon, DefaultButton);
    }

    /// <summary>
    /// Runtime translation. English text is used as the key, translations live in Resources\Lang\{code}.json
    /// (embedded in the exe, a Lang\{code}.json next to the exe overrides it so translations can be edited locally).
    /// </summary>
    public static class Lang
    {
        public static readonly Language[] Languages =
        {
            new Language("en", "English", "gb"),
            new Language("pt-PT", "Português (Portugal)", "pt"),
            new Language("es", "Español", "es"),
            new Language("fr", "Français", "fr"),
            new Language("de", "Deutsch", "de"),
        };

        public static Language Current { get; private set; } = Languages[0];
        public static event EventHandler LanguageChanged;

        private static Dictionary<string, string> Table = new Dictionary<string, string>();
        private static List<(Regex Pattern, string Translation)> Templates = new List<(Regex, string)>();
        private static readonly ConditionalWeakTable<object, Dictionary<string, (string Original, string Applied)>> Originals = new ConditionalWeakTable<object, Dictionary<string, (string, string)>>();
        private static readonly ConditionalWeakTable<ListControl, object> FormattedLists = new ConditionalWeakTable<ListControl, object>();

        public static void Initialize()
        {
            string Code = AccountManager.General?.Get("Language");

            if (Code != null && Code.StartsWith("pt")) Code = "pt-PT"; // pt-BR was removed

            if (string.IsNullOrEmpty(Code) || !Languages.Any(L => L.Code == Code))
                Code = Detect();

            SetLanguage(Code, false);
        }

        private static string Detect()
        {
            CultureInfo Culture = CultureInfo.CurrentUICulture;

            if (Culture.TwoLetterISOLanguageName == "pt") return "pt-PT"; // every Portuguese variant uses pt-PT

            return Languages.FirstOrDefault(L => L.Code == Culture.TwoLetterISOLanguageName)?.Code ?? "en";
        }

        public static void SetLanguage(string Code, bool Save = true)
        {
            Current = Languages.FirstOrDefault(L => L.Code == Code) ?? Languages[0];
            Table = LoadTable(Current.Code);
            Templates = Table.Where(Entry => Entry.Key.Contains("{0}"))
                .Select(Entry => (new Regex("^" + Regex.Replace(Regex.Escape(Entry.Key).Replace("\\{", "{"), @"\{(\d+)}", "(?<p$1>.*?)") + "$", RegexOptions.Singleline), Entry.Value))
                .ToList();

            if (Save && AccountManager.General != null)
            {
                AccountManager.General.Set("Language", Current.Code);
                AccountManager.IniSettings.Save("RAMSettings.ini");
            }

            LanguageChanged?.Invoke(null, EventArgs.Empty);
        }

        private static Dictionary<string, string> LoadTable(string Code)
        {
            try
            {
                string LocalFile = Path.Combine(Environment.CurrentDirectory, "Lang", Code + ".json");
                string Json = null;

                if (File.Exists(LocalFile))
                    Json = File.ReadAllText(LocalFile);
                else
                {
                    Assembly assembly = Assembly.GetExecutingAssembly();
                    string ResourceName = assembly.GetManifestResourceNames().FirstOrDefault(Name => Name.EndsWith($".Lang.{Code}.json", StringComparison.OrdinalIgnoreCase));

                    if (ResourceName != null)
                        using (var reader = new StreamReader(assembly.GetManifestResourceStream(ResourceName)))
                            Json = reader.ReadToEnd();
                }

                if (!string.IsNullOrEmpty(Json))
                    return JsonConvert.DeserializeObject<Dictionary<string, string>>(Json) ?? new Dictionary<string, string>();
            }
            catch (Exception x) { Program.Logger.Error($"Failed to load language {Code}: {x}"); }

            return new Dictionary<string, string>();
        }

        /// <summary>Translate a piece of English text</summary>
        public static string T(string Text)
        {
            if (string.IsNullOrEmpty(Text)) return Text;
            if (Table.TryGetValue(Text, out string Translated)) return Translated;

            string Trimmed = Text.Trim();

            if (Trimmed.Length != Text.Length && Table.TryGetValue(Trimmed, out Translated))
                return Text.Replace(Trimmed, Translated);

            // Texts built from a template, e.g. "Are you sure you want to remove {0} accounts?"
            foreach (var (Pattern, Translation) in Templates)
            {
                Match match = Pattern.Match(Text);

                if (!match.Success) continue;

                return Regex.Replace(Translation, @"\{(\d+)}", Placeholder => match.Groups["p" + Placeholder.Groups[1].Value] is Group Value && Value.Success ? T(Value.Value) : Placeholder.Value);
            }

            return Text;
        }

        /// <summary>Translate a format string, then fill it in</summary>
        public static string F(string Format, params object[] Args) => string.Format(T(Format), Args);

        public static Image FlagImage(Language language, int Height) => ModernUI.Flag(language.Flag, Height);

        #region Applying to controls

        public static void Apply(Control Root)
        {
            if (Root == null) return;

            ApplyControl(Root);
        }

        private static void Translate(object Owner, string Property, Func<string> Get, Action<string> Set)
        {
            string Current = Get();

            if (string.IsNullOrEmpty(Current)) return;

            var Dict = Originals.GetOrCreateValue(Owner);

            // If the text was changed by code since the last time it was translated, treat the new text as the original
            string Original = Dict.TryGetValue(Property, out var Entry) && Entry.Applied == Current ? Entry.Original : Current;
            string Translated = T(Original);

            if (Translated != Current) Set(Translated);

            Dict[Property] = (Original, Translated);
        }

        private static void ApplyControl(Control control)
        {
            bool UserContent = control is TextBoxBase || control is ComboBox || control is NumericUpDown || control is ListBox || control is ObjectListView;

            if (!UserContent)
                Translate(control, "Text", () => control.Text, Value => control.Text = Value);

            if (control is ListControl List && !(control is ComboBox Box && Box.DropDownStyle == ComboBoxStyle.DropDown))
                ApplyList(List);

            if (control is ObjectListView View)
                foreach (OLVColumn Column in View.AllColumns)
                    Translate(Column, "Text", () => Column.Text, Value => Column.Text = Value);

            if (control.ContextMenuStrip != null)
                ApplyItems(control.ContextMenuStrip.Items);

            if (control is MenuButton Menu && Menu.Menu != null)
                ApplyItems(Menu.Menu.Items);

            if (control is Form || control is UserControl)
                ApplyComponents(control);

            foreach (Control Child in control.Controls)
                ApplyControl(Child);
        }

        private static void ApplyList(ListControl List)
        {
            if (!FormattedLists.TryGetValue(List, out _))
            {
                FormattedLists.Add(List, null);

                List.FormattingEnabled = true;
                List.Format += (s, e) => { if (e.ListItem is string Item) e.Value = T(Item); };
            }
            else
            { // force the items to be formatted again
                List.FormattingEnabled = false;
                List.FormattingEnabled = true;
            }
        }

        /// <summary>Translates components that aren't part of the Controls tree (menus, tooltips)</summary>
        private static void ApplyComponents(Control Owner)
        {
            var Fields = Owner.GetType().GetFields(BindingFlags.Instance | BindingFlags.NonPublic | BindingFlags.Public);
            var Controls = AllControls(Owner).ToList();

            foreach (FieldInfo Field in Fields)
            {
                object Value = Field.GetValue(Owner);

                if (Value is ContextMenuStrip Strip)
                    ApplyItems(Strip.Items);
                else if (Value is ToolTip Tip)
                    foreach (Control control in Controls)
                    {
                        Control Target = control;
                        Translate(Target, "ToolTip" + Tip.GetHashCode(), () => Tip.GetToolTip(Target), Text => Tip.SetToolTip(Target, Text));
                    }
            }
        }

        public static void ApplyItems(ToolStripItemCollection Items)
        {
            foreach (ToolStripItem Item in Items)
            {
                Translate(Item, "Text", () => Item.Text, Value => Item.Text = Value);
                Translate(Item, "ToolTipText", () => Item.ToolTipText, Value => Item.ToolTipText = Value);

                if (Item is ToolStripDropDownItem DropDown)
                    ApplyItems(DropDown.DropDownItems);
            }
        }

        private static IEnumerable<Control> AllControls(Control Root)
        {
            yield return Root;

            foreach (Control Child in Root.Controls)
                foreach (Control Sub in AllControls(Child))
                    yield return Sub;
        }

        #endregion
    }
}
