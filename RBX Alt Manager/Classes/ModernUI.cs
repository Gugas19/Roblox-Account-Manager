using BrightIdeasSoftware;
using RBX_Alt_Manager.Forms;
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Linq;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace RBX_Alt_Manager.Classes
{
    /// <summary>
    /// Modern look for the WinForms UI: rounded flat buttons with icons, dark menus/scrollbars, styled lists and Windows 11 window chrome.
    /// Colors come from the theme (ThemeEditor) so custom themes keep working.
    /// Icons are Google's Material Icons (Apache 2.0, see Resources\ThirdParty), flags come from flagcdn.com, both embedded in the exe.
    /// </summary>
    public static class ModernUI
    {
        public const string FontName = "Segoe UI";

        #region Native

        [DllImport("dwmapi.dll")]
        private static extern int DwmSetWindowAttribute(IntPtr hwnd, int attr, ref int attrValue, int attrSize);

        [DllImport("uxtheme.dll", CharSet = CharSet.Unicode)]
        private static extern int SetWindowTheme(IntPtr hWnd, string pszSubAppName, string pszSubIdList);

        [DllImport("user32.dll", CharSet = CharSet.Unicode)]
        private static extern IntPtr SendMessage(IntPtr hWnd, int Msg, IntPtr wParam, string lParam);

        [DllImport("user32.dll")]
        private static extern IntPtr SendMessage(IntPtr hWnd, int Msg, IntPtr wParam, IntPtr lParam);

        private const int DWMWA_WINDOW_CORNER_PREFERENCE = 33, DWMWA_BORDER_COLOR = 34, DWMWA_CAPTION_COLOR = 35, DWMWA_TEXT_COLOR = 36;
        private const int EM_SETMARGINS = 0xD3, EC_LEFTMARGIN = 1, EC_RIGHTMARGIN = 2, EM_SETCUEBANNER = 0x1501;

        private static int ColorRef(Color c) => c.R | (c.G << 8) | (c.B << 16);

        #endregion

        #region Colors

        public static Color Accent => ThemeEditor.AccentColor;
        public static bool IsDark => ThemeEditor.FormsBackground.GetBrightness() < 0.5f;
        public static Color Card => Shift(ThemeEditor.FormsBackground, 0.035f);
        public static Color Muted => Mix(ThemeEditor.FormsForeground, ThemeEditor.FormsBackground, 0.42f);
        public static Color Border => Shift(ThemeEditor.FormsBackground, 0.11f);

        public static Color Mix(Color a, Color b, float t) => Color.FromArgb(255, (int)(a.R + (b.R - a.R) * t), (int)(a.G + (b.G - a.G) * t), (int)(a.B + (b.B - a.B) * t));

        /// <summary>Lightens dark colors and darkens light colors</summary>
        public static Color Shift(Color c, float Amount) => c.GetBrightness() < 0.5f ? Mix(c, Color.White, Amount) : Mix(c, Color.Black, Amount);

        public static Color ContrastText(Color Background) => (0.299 * Background.R + 0.587 * Background.G + 0.114 * Background.B) / 255 > 0.62 ? Color.FromArgb(20, 20, 24) : Color.White;

        #endregion

        #region Icons & Flags

        private static readonly Dictionary<string, Bitmap> RawImages = new Dictionary<string, Bitmap>();
        private static readonly Dictionary<string, Bitmap> ImageCache = new Dictionary<string, Bitmap>();

        private static Bitmap LoadEmbedded(string Folder, string Name)
        {
            string Key = $"{Folder}.{Name}";

            if (RawImages.TryGetValue(Key, out Bitmap Cached)) return Cached;

            Assembly assembly = Assembly.GetExecutingAssembly();
            string Resource = assembly.GetManifestResourceNames().FirstOrDefault(R => R.EndsWith($".{Folder}.{Name}.png", StringComparison.OrdinalIgnoreCase));
            Bitmap Image = null;

            if (Resource != null)
                using (var Stream = assembly.GetManifestResourceStream(Resource))
                    Image = new Bitmap(Stream);

            RawImages[Key] = Image;

            return Image;
        }

        private static Bitmap Resize(Image Source, int Width, int Height, int CornerRadius = 0)
        {
            Bitmap Scaled = new Bitmap(Width, Height, System.Drawing.Imaging.PixelFormat.Format32bppArgb);

            using (Graphics g = Graphics.FromImage(Scaled))
            {
                g.InterpolationMode = InterpolationMode.HighQualityBicubic;
                g.PixelOffsetMode = PixelOffsetMode.HighQuality;
                g.CompositingQuality = CompositingQuality.HighQuality;
                g.DrawImage(Source, new Rectangle(0, 0, Width, Height));
            }

            if (CornerRadius <= 0) return Scaled;

            Bitmap Result = new Bitmap(Width, Height, System.Drawing.Imaging.PixelFormat.Format32bppArgb);

            using (Graphics g = Graphics.FromImage(Result))
            using (var Brush = new TextureBrush(Scaled, WrapMode.Clamp))
            using (var Path = RoundedRect(new RectangleF(0, 0, Width, Height), CornerRadius))
            {
                g.SmoothingMode = SmoothingMode.AntiAlias;
                g.PixelOffsetMode = PixelOffsetMode.HighQuality;
                g.FillPath(Brush, Path);
            }

            Scaled.Dispose();

            return Result;
        }

        /// <summary>Material icon recolored to the given color</summary>
        public static Bitmap Icon(string Name, int Size, Color Color)
        {
            if (string.IsNullOrEmpty(Name) || Size <= 0) return null;

            string Key = $"icon:{Name}:{Size}:{Color.ToArgb()}";

            if (ImageCache.TryGetValue(Key, out Bitmap Cached)) return Cached;

            Bitmap Raw = LoadEmbedded("Icons", Name);

            if (Raw == null) return null;

            Bitmap Result = Resize(Raw, Size, Size);

            for (int x = 0; x < Result.Width; x++)
                for (int y = 0; y < Result.Height; y++)
                {
                    Color Pixel = Result.GetPixel(x, y);

                    if (Pixel.A > 0) Result.SetPixel(x, y, Color.FromArgb(Pixel.A, Color));
                }

            return ImageCache[Key] = Result;
        }

        public static Bitmap Flag(string Code, int Height)
        {
            string Key = $"flag:{Code}:{Height}";

            if (ImageCache.TryGetValue(Key, out Bitmap Cached)) return Cached;

            Bitmap Raw = LoadEmbedded("Flags", Code);

            if (Raw == null) return null;

            return ImageCache[Key] = Resize(Raw, (int)Math.Round(Height * 4 / 3f), Height, Math.Max(2, Height / 6));
        }

        public static int IconSize => (int)(16 * Program.Scale);

        #endregion

        #region Icon assignments

        private static readonly Dictionary<string, string> ButtonIcons = new Dictionary<string, string>
        {
            // Main window
            { "Add", "person_add" }, { "Remove", "person_remove" }, { "OpenBrowser", "public" }, { "JoinServer", "play_arrow" },
            { "Follow", "person_search" }, { "SetAlias", "label" }, { "SetDescription", "edit_note" }, { "ServerList", "dns" },
            { "BrowserButton", "manage_accounts" }, { "EditTheme", "palette" }, { "ConfigButton", "settings" }, { "SaveToAccount", "save" },
            { "DefaultEncryptionButton", "shield" }, { "PasswordEncryptionButton", "lock" }, { "UnlockButton", "lock_open" }, { "SetPasswordButton", "check" },
            // Utilities window
            { "RefreshServers", "refresh" }, { "SearchPlayer", "person_search" }, { "Search", "search" }, { "Favorite", "star" },
            { "GetUniverseID", "search" }, { "ViewUniverse", "travel_explore" }, { "ViewOutfits", "checkroom" }, { "WearCustomButton", "checkroom" },
            { "OpenLogsButton", "folder" },
            // Other windows
            { "EncryptionSelectionButton", "lock" }, { "ImportButton", "download" }, { "Delete", "close" }, { "BuyButton", "bolt" },
            { "SetBG", "palette" }, { "SetFG", "palette" }, { "SetBorder", "palette" }, { "ChangeStyle", "tune" }, { "HideHeaders", "visibility_off" },
            { "ToggleDarkTopBar", "tune" }, { "ToggleTransparentBG", "tune" },
            { "Block", "block" }, { "SetDisplayName", "badge" }, { "AddFriend", "person_add" },
        };

        private static readonly Dictionary<Type, Dictionary<string, string>> FormButtonIcons = new Dictionary<Type, Dictionary<string, string>>
        {
            { typeof(AccountUtils), new Dictionary<string, string> { { "button1", "key" }, { "button2", "logout" }, { "button3", "edit" }, { "button7", "lock_open" } } },
            { typeof(ArgumentsForm), new Dictionary<string, string> { { "button1", "check" } } },
            { typeof(AccountFields), new Dictionary<string, string> { { "Add", null } } }, // its text is already "+"
        };

        private static readonly Dictionary<string, string> MenuIcons = new Dictionary<string, string>
        {
            { "addAccountsToolStripMenuItem", "person_add" }, { "removeAccountToolStripMenuItem", "person_remove" }, { "copyToolStripMenuItem", "content_copy" },
            { "copyUsernameToolStripMenuItem", "person" }, { "copyPasswordToolStripMenuItem", "key" }, { "copyUserPassComboToolStripMenuItem", "vpn_key" },
            { "copyProfileToolStripMenuItem", "link" }, { "copyUserIdToolStripMenuItem", "badge" }, { "sortAlphabeticallyToolStripMenuItem", "sort_by_alpha" },
            { "quickLogInToolStripMenuItem", "qr_code_2" }, { "moveGroupUpToolStripMenuItem", "folder" }, { "toggleToolStripMenuItem", "unfold_more" },
            { "moveToToolStripMenuItem", "drive_file_move" }, { "copyGroupToolStripMenuItem", "content_copy" }, { "infoToolStripMenuItem", "info" },
            { "groupsToolStripMenuItem", "help" }, { "infoToolStripMenuItem1", "info" }, { "viewFieldsToolStripMenuItem", "list" },
            { "ShowDetailsToolStripMenuItem", "description" }, { "getAuthenticationTicketToolStripMenuItem", "vpn_key" }, { "copySecurityTokenToolStripMenuItem", "cookie" },
            { "copyRbxplayerLinkToolStripMenuItem", "link" }, { "copyAppLinkToolStripMenuItem", "open_in_new" },
            { "manualToolStripMenuItem", "person_add" }, { "bulkUserPassToolStripMenuItem", "group_add" }, { "byCookieToolStripMenuItem", "cookie" },
            { "customURLJSToolStripMenuItem", "code" }, { "customURLToolStripMenuItem", "public" }, { "URLJSToolStripMenuItem", "code" },
            { "joinGroupToolStripMenuItem", "group_add" },
            { "joinServerToolStripMenuItem", "play_arrow" }, { "copyJobIdToolStripMenuItem", "content_copy" }, { "loadRegionToolStripMenuItem", "map" },
            { "joinGameToolStripMenuItem", "play_arrow" }, { "addToFavoritesToolStripMenuItem", "star" }, { "copyPlaceIdToolStripMenuItem1", "content_copy" },
            { "toolStripMenuItem1", "play_arrow" }, { "toolStripMenuItem2", "edit" }, { "removeToolStripMenuItem", "delete" }, { "copyPlaceIDToolStripMenuItem", "content_copy" },
            { "copyPlaceLinkToolStripMenuItem", "link" }, { "copyPlaceIdToolStripMenuItem", "content_copy" }, { "copyNameToolStripMenuItem", "content_copy" },
            { "copyPlaceDetailsToolStripMenuItem", "info" }, { "wearAvatarToolStripMenuItem", "checkroom" }, { "copyAvatarJSONToolStripMenuItem", "content_copy" },
        };

        private static readonly HashSet<string> PrimaryButtons = new HashSet<string> { "JoinServer", "UnlockButton", "SetPasswordButton", "PasswordEncryptionButton", "ImportButton", "BuyButton" };

        public static void SetIcon(Button button, string Icon)
        {
            GetState(button).Icon = Icon;
            button.Invalidate();
        }

        /// <summary>Image drawn on the button that the theme won't recolor (flags, logos)</summary>
        public static void SetImage(Button button, Image Image)
        {
            GetState(button).Picture = Image;
            button.Invalidate();
        }

        public static void SetPrimary(Button button, bool Primary = true)
        {
            GetState(button).Primary = Primary;
            button.Invalidate();
        }

        #endregion

        #region Window & fonts

        public static void StyleWindow(IntPtr Handle)
        {
            if (Handle == IntPtr.Zero) return;

            try
            {
                int Round = 2; // DWMWCP_ROUND, Windows 11 only (ignored elsewhere)
                int Caption = ColorRef(ThemeEditor.FormsBackground);
                int Text = ColorRef(ThemeEditor.FormsForeground);
                int WindowBorder = ColorRef(Border);

                DwmSetWindowAttribute(Handle, DWMWA_WINDOW_CORNER_PREFERENCE, ref Round, sizeof(int));
                DwmSetWindowAttribute(Handle, DWMWA_CAPTION_COLOR, ref Caption, sizeof(int));
                DwmSetWindowAttribute(Handle, DWMWA_TEXT_COLOR, ref Text, sizeof(int));
                DwmSetWindowAttribute(Handle, DWMWA_BORDER_COLOR, ref WindowBorder, sizeof(int));
            }
            catch { }
        }

        /// <summary>Swaps the old default fonts for Segoe UI, keeping their size & style</summary>
        public static void PrepareFonts(Control Root)
        {
            Font Current = Root.Font;

            if (Current.FontFamily.Name != FontName && (Root.Parent == null || Root is Form || Current != Root.Parent.Font))
                Root.Font = new Font(FontName, Current.Size, Current.Style, Current.Unit);

            foreach (Control Child in Root.Controls)
                PrepareFonts(Child);
        }

        #endregion

        #region Applying

        private static readonly Dictionary<TextBox, string> CueBanners = new Dictionary<TextBox, string>();

        /// <summary>Sets a placeholder text shown while the TextBox is empty (translated automatically)</summary>
        public static void SetPlaceholder(TextBox Box, string Text)
        {
            CueBanners[Box] = Text;

            void Set() => SendMessage(Box.Handle, EM_SETCUEBANNER, (IntPtr)1, Lang.T(Text));

            if (Box.IsHandleCreated) Set(); else Box.HandleCreated += (s, e) => Set();
        }

        /// <summary>Styles a whole form/control tree, then translates it</summary>
        public static void Apply(Control Root)
        {
            if (Root == null) return;

            if (Root is Form form)
            {
                if (form.IsHandleCreated) StyleWindow(form.Handle);
                else form.HandleCreated += (s, e) => StyleWindow(form.Handle);
            }

            PrepareFonts(Root);
            StyleTree(Root);
            Lang.Apply(Root);
            FitButtons(Root);

            foreach (var Cue in CueBanners)
                if (!Cue.Key.IsDisposed && Cue.Key.IsHandleCreated)
                    SendMessage(Cue.Key.Handle, EM_SETCUEBANNER, (IntPtr)1, Lang.T(Cue.Value));
        }

        /// <summary>
        /// Flat check boxes draw a white box with the tick in the text color, so on a dark theme the tick is invisible.
        /// The box is redrawn on top: dark when unchecked, accent colored with a contrasting tick when checked.
        /// </summary>
        private static void PaintCheckBox(object sender, PaintEventArgs e)
        {
            var Check = (CheckBox)sender;
            if (Check.Appearance != Appearance.Normal) return;

            float Dpi = Check.DeviceDpi / 96f;
            int Size = (int)Math.Round(13 * Dpi);
            var Box = new Rectangle(Check.Padding.Left, (Check.ClientSize.Height - Size) / 2, Size, Size);

            if (Check.CheckAlign == ContentAlignment.TopLeft) Box.Y = Check.Padding.Top + 1;

            Color Back = Check.BackColor.A == 255 ? Check.BackColor : (Check.Parent?.BackColor ?? ThemeEditor.FormsBackground);
            bool On = Check.CheckState != CheckState.Unchecked;
            Color Fill = On ? Accent : ThemeEditor.TextBoxesBackground;
            Color Edge = On ? Accent : InputBorder;

            if (!Check.Enabled)
            {
                Fill = Mix(Fill, Back, 0.5f);
                Edge = Mix(Edge, Back, 0.5f);
            }

            e.Graphics.SmoothingMode = System.Drawing.Drawing2D.SmoothingMode.AntiAlias;

            using (var Erase = new SolidBrush(Back))
                e.Graphics.FillRectangle(Erase, Rectangle.Inflate(Box, 1, 1));

            using (var Path = RoundedRect(new RectangleF(Box.X + 0.5f, Box.Y + 0.5f, Box.Width - 1, Box.Height - 1), 3 * Dpi))
            {
                using (var Brush = new SolidBrush(Fill)) e.Graphics.FillPath(Brush, Path);
                using (var Pen = new Pen(Edge)) e.Graphics.DrawPath(Pen, Path);
            }

            if (Check.CheckState == CheckState.Checked)
                using (var Pen = new Pen(ContrastText(Fill), 1.8f * Dpi) { StartCap = System.Drawing.Drawing2D.LineCap.Round, EndCap = System.Drawing.Drawing2D.LineCap.Round, LineJoin = System.Drawing.Drawing2D.LineJoin.Round })
                    e.Graphics.DrawLines(Pen, new[]
                    {
                        new PointF(Box.X + Size * 0.25f, Box.Y + Size * 0.52f),
                        new PointF(Box.X + Size * 0.43f, Box.Y + Size * 0.70f),
                        new PointF(Box.X + Size * 0.76f, Box.Y + Size * 0.32f)
                    });
            else if (Check.CheckState == CheckState.Indeterminate)
                using (var Brush = new SolidBrush(ContrastText(Fill)))
                    e.Graphics.FillRectangle(Brush, Box.X + Size * 0.28f, Box.Y + Size * 0.45f, Size * 0.44f, Math.Max(2, Size * 0.12f));
        }

        private static void StyleTree(Control control)
        {
            if (control is Label && control.RightToLeft == RightToLeft.Yes) // used in the designer to right-align, but it moves punctuation like "(min)" or "max." around
                control.RightToLeft = RightToLeft.No;

            switch (control)
            {
                case Button button:
                    StyleButton(button);
                    break;

                case CheckBox Check:
                    Check.FlatStyle = FlatStyle.Flat;
                    Check.FlatAppearance.BorderColor = Shift(ThemeEditor.TextBoxesBackground, 0.25f);
                    Check.FlatAppearance.CheckedBackColor = Accent;
                    Check.FlatAppearance.MouseOverBackColor = Shift(ThemeEditor.TextBoxesBackground, 0.12f);
                    Check.Cursor = Cursors.Hand;

                    if (Check.Appearance == Appearance.Normal && !WatchedContainers.TryGetValue(Check, out _))
                    {
                        WatchedContainers.Add(Check, null);
                        Check.Paint += PaintCheckBox;
                    }
                    break;

                case ComboBox Combo:
                    Combo.FlatStyle = FlatStyle.Flat;
                    Combo.BackColor = ThemeEditor.TextBoxesBackground;
                    Combo.ForeColor = ThemeEditor.TextBoxesForeground;
                    DarkScrollbars(Combo);
                    break;

                case NumericUpDown Number:
                    Number.BackColor = ThemeEditor.TextBoxesBackground;
                    Number.ForeColor = ThemeEditor.TextBoxesForeground;
                    Number.BorderStyle = BorderStyle.FixedSingle;

                    if (!WatchedContainers.TryGetValue(Number, out _))
                    {
                        WatchedContainers.Add(Number, null);
                        Number.Paint += (s, e) => { using (var Pen = new Pen(InputBorder)) e.Graphics.DrawRectangle(Pen, 0, 0, Number.Width - 1, Number.Height - 1); };
                    }

                    Number.Invalidate();
                    break;

                case ListBox List:
                    List.BackColor = ThemeEditor.TextBoxesBackground;
                    List.ForeColor = ThemeEditor.TextBoxesForeground;
                    List.BorderStyle = BorderStyle.None;
                    DarkScrollbars(List);
                    break;

                case TextBox Box:
                    if (Box.BorderStyle != BorderStyle.None && !(Box is BorderedTextBox) && !TextBoxBorders.TryGetValue(Box, out _))
                    {
                        Box.BorderStyle = BorderStyle.FixedSingle; // the 3D border looks white on dark themes
                        TextBoxBorders.Add(Box, new TextBoxBorder(Box));
                    }

                    void Margins() => SendMessage(Box.Handle, EM_SETMARGINS, (IntPtr)(EC_LEFTMARGIN | EC_RIGHTMARGIN), (IntPtr)((int)(5 * Program.Scale) | ((int)(5 * Program.Scale) << 16)));
                    if (Box.IsHandleCreated) Margins(); else Box.HandleCreated += (s, e) => Margins();
                    if (Box.Multiline) DarkScrollbars(Box);
                    break;

                case RichTextBox Rich:
                    DarkScrollbars(Rich);

                    if (Rich.BorderStyle != BorderStyle.None && !(Rich is BorderedRichTextBox) && Rich.Parent != null && !TextBoxBorders.TryGetValue(Rich, out _))
                    { // the rich edit control repaints its own border, so it gets none and the parent draws one around it
                        TextBoxBorders.Add(Rich, null);
                        Rich.BorderStyle = BorderStyle.None;

                        Control Parent = Rich.Parent;
                        Parent.Paint += (s, e) =>
                        {
                            if (!Rich.Visible) return;

                            using (var Pen = new Pen(InputBorder))
                                e.Graphics.DrawRectangle(Pen, Rectangle.Inflate(Rich.Bounds, 1, 1));
                        };
                        Rich.SizeChanged += (s, e) => Parent.Invalidate();
                        Rich.LocationChanged += (s, e) => Parent.Invalidate();
                    }
                    break;

                case ObjectListView View:
                    StyleList(View);
                    break;

                case TabPage Page:
                    Page.BackColor = ThemeEditor.FormsBackground;
                    Page.ForeColor = ThemeEditor.FormsForeground;
                    break;

                case TabControl Tabs:
                    Tabs.Padding = new Point((int)(12 * Program.Scale), (int)(6 * Program.Scale));
                    Tabs.Invalidate();
                    break;

                case LinkLabel Link:
                    Link.LinkColor = Accent;
                    Link.ActiveLinkColor = Shift(Accent, 0.2f);
                    break;

                case Label Section when Section.Name != null && Section.Name.StartsWith("Section"):
                    Section.ForeColor = Muted;
                    Section.BackColor = Color.Transparent;
                    break;

                case CardPanel cardPanel:
                    cardPanel.BackColor = cardPanel.Fill?.Invoke() ?? Card;
                    break;

                case ScrollableControl Scrollable when Scrollable.AutoScroll:
                    DarkScrollbars(Scrollable);
                    break;
            }

            if ((control is Panel || control is UserControl) && !WatchedContainers.TryGetValue(control, out _))
            {
                WatchedContainers.Add(control, null);

                control.ControlAdded += (s, e) =>
                {
                    if (e.Control == null || e.Control.IsDisposed) return;

                    PrepareFonts(e.Control);
                    StyleTree(e.Control);
                    Lang.Apply(e.Control);
                };
            }

            if (control.ContextMenuStrip != null) StyleMenu(control.ContextMenuStrip);
            if (control is MenuButton Menu && Menu.Menu != null) StyleMenu(Menu.Menu);

            if (control is Form || control is UserControl)
                foreach (FieldInfo Field in control.GetType().GetFields(BindingFlags.Instance | BindingFlags.NonPublic | BindingFlags.Public))
                    if (Field.GetValue(control) is ContextMenuStrip Strip)
                        StyleMenu(Strip);

            foreach (Control Child in control.Controls)
                StyleTree(Child);
        }

        private static readonly ConditionalWeakTable<Control, object> WatchedContainers = new ConditionalWeakTable<Control, object>();
        private static readonly ConditionalWeakTable<TextBoxBase, TextBoxBorder> TextBoxBorders = new ConditionalWeakTable<TextBoxBase, TextBoxBorder>();

        public static Color InputBorder => Shift(ThemeEditor.TextBoxesBackground, 0.14f);

        [DllImport("user32.dll")] private static extern IntPtr GetWindowDC(IntPtr hWnd);
        [DllImport("user32.dll")] private static extern int ReleaseDC(IntPtr hWnd, IntPtr hDC);

        /// <summary>Paints the 1px border of a FixedSingle TextBox in the theme color instead of the system's bright one</summary>
        private class TextBoxBorder : NativeWindow
        {
            private readonly TextBoxBase Box;

            public TextBoxBorder(TextBoxBase Box)
            {
                this.Box = Box;

                if (Box.IsHandleCreated) AssignHandle(Box.Handle);

                Box.HandleCreated += (s, e) => AssignHandle(Box.Handle);
                Box.HandleDestroyed += (s, e) => ReleaseHandle();
            }

            protected override void WndProc(ref Message m)
            {
                base.WndProc(ref m);

                if (m.Msg == 0x85 || m.Msg == 0xF) // WM_NCPAINT, WM_PAINT
                {
                    IntPtr DC = GetWindowDC(Handle);

                    if (DC == IntPtr.Zero) return;

                    using (Graphics g = Graphics.FromHdc(DC))
                    using (var Pen = new Pen(InputBorder))
                        g.DrawRectangle(Pen, 0, 0, Box.Width - 1, Box.Height - 1);

                    ReleaseDC(Handle, DC);
                }
            }
        }

        private static void DarkScrollbars(Control control)
        {
            void Set() => SetWindowTheme(control.Handle, IsDark ? "DarkMode_Explorer" : "Explorer", null);

            if (control.IsHandleCreated) Set(); else control.HandleCreated += (s, e) => Set();
        }

        #endregion

        #region Buttons

        private class ButtonState
        {
            public bool Hooked, Hover, Pressed, Primary;
            public string Icon;
            public Image Picture;
        }

        private static readonly ConditionalWeakTable<Button, ButtonState> ButtonStates = new ConditionalWeakTable<Button, ButtonState>();

        private static ButtonState GetState(Button button) => ButtonStates.GetValue(button, b => new ButtonState { Primary = PrimaryButtons.Contains(b.Name) });

        private static void StyleButton(Button button)
        {
            ButtonState State = GetState(button);

            if (State.Icon == null)
            {
                Form Owner = button.FindForm();

                if (Owner != null && FormButtonIcons.TryGetValue(Owner.GetType(), out var FormIcons) && FormIcons.TryGetValue(button.Name, out string FormIcon))
                    State.Icon = FormIcon;
                else if (ButtonIcons.TryGetValue(button.Name, out string Icon))
                    State.Icon = Icon;
            }

            button.FlatStyle = FlatStyle.Flat;
            button.FlatAppearance.BorderSize = 0;
            button.FlatAppearance.MouseOverBackColor = Color.Transparent;
            button.FlatAppearance.MouseDownBackColor = Color.Transparent;
            button.Cursor = Cursors.Hand;

            if (!State.Hooked)
            {
                State.Hooked = true;

                typeof(Control).GetProperty("DoubleBuffered", BindingFlags.Instance | BindingFlags.NonPublic)?.SetValue(button, true);

                button.Paint += PaintButton;
                button.MouseEnter += (s, e) => { State.Hover = true; button.Invalidate(); };
                button.MouseLeave += (s, e) => { State.Hover = false; State.Pressed = false; button.Invalidate(); };
                button.MouseDown += (s, e) => { if (e.Button == MouseButtons.Left) { State.Pressed = true; button.Invalidate(); } };
                button.MouseUp += (s, e) => { State.Pressed = false; button.Invalidate(); };
                button.EnabledChanged += (s, e) => button.Invalidate();
            }

            button.Invalidate();
        }

        private static Color EffectiveBackColor(Control control)
        {
            while (control != null && control.BackColor.A < 255)
                control = control.Parent;

            return control?.BackColor ?? ThemeEditor.FormsBackground;
        }

        private static void PaintButton(object sender, PaintEventArgs e)
        {
            Button button = (Button)sender;
            ButtonState State = GetState(button);
            Graphics g = e.Graphics;

            Color ParentBack = EffectiveBackColor(button.Parent);
            Color Back = State.Primary ? Accent : ThemeEditor.ButtonsBackground;

            if (!State.Primary && Back.ToArgb() == ParentBack.ToArgb()) Back = Shift(Back, 0.06f); // keep buttons visible on cards with the same color

            if (!button.Enabled) Back = Mix(Back, ParentBack, 0.55f);
            else if (State.Pressed) Back = Shift(Back, 0.16f);
            else if (State.Hover) Back = Shift(Back, 0.08f);

            Color Fore = State.Primary ? ContrastText(Back) : ThemeEditor.ButtonsForeground;

            if (!button.Enabled) Fore = Mix(Fore, Back, 0.5f);

            g.Clear(ParentBack);
            g.SmoothingMode = SmoothingMode.AntiAlias;

            float Radius = Math.Min(6f * Program.Scale, button.Height / 2f);
            RectangleF Bounds = new RectangleF(0.5f, 0.5f, button.Width - 1.5f, button.Height - 1.5f);

            using (var Path = RoundedRect(Bounds, Radius))
            {
                using (var Brush = new SolidBrush(Back))
                    g.FillPath(Brush, Path);

                if (!State.Primary)
                    using (var Pen = new Pen(Shift(Back, 0.07f)))
                        g.DrawPath(Pen, Path);
            }

            g.SmoothingMode = SmoothingMode.None;

            int Reserved = button is MenuButton menuButton && menuButton.Menu != null ? (int)(18 * Program.Scale) : 0;
            int Pad = (int)(8 * Program.Scale);
            int IconPx = Math.Min(IconSize, button.Height - (int)(6 * Program.Scale));
            Image Picture = State.Picture ?? (State.Icon != null ? Icon(State.Icon, IconPx, Fore) : button.Image);
            string Text = button.Text;

            TextFormatFlags Flags = TextFormatFlags.VerticalCenter | TextFormatFlags.SingleLine | TextFormatFlags.EndEllipsis | TextFormatFlags.HidePrefix | TextFormatFlags.NoPadding;
            int Available = button.Width - Reserved - Pad * 2;
            Size TextSize = string.IsNullOrEmpty(Text) ? System.Drawing.Size.Empty : TextRenderer.MeasureText(g, Text, button.Font, new Size(int.MaxValue, button.Height), Flags);
            int ImageWidth = Picture != null ? Math.Min(Picture.Width, IconPx) : 0;
            int ImageHeight = Picture != null ? (int)(Picture.Height * ((float)ImageWidth / Picture.Width)) : 0;
            int Gap = Picture != null && TextSize.Width > 0 ? (int)(6 * Program.Scale) : 0;
            int Total = ImageWidth + Gap + TextSize.Width;
            int X = Total <= Available ? Pad + (Available - Total) / 2 : Pad;

            if (Picture != null)
            {
                g.InterpolationMode = InterpolationMode.HighQualityBicubic;
                g.DrawImage(Picture, new Rectangle(X, (button.Height - ImageHeight) / 2, ImageWidth, ImageHeight));
                X += ImageWidth + Gap;
            }

            if (TextSize.Width > 0)
                TextRenderer.DrawText(g, Text, button.Font, new Rectangle(X, 0, Math.Max(0, button.Width - Reserved - Pad - X), button.Height), Fore, Flags);
        }

        private static readonly MethodInfo GetControlState = typeof(Control).GetMethod("GetState", BindingFlags.Instance | BindingFlags.NonPublic, null, new[] { typeof(int) }, null);

        /// <summary>The control's own Visible flag, Control.Visible is false for everything while the window is hidden</summary>
        private static bool IsVisibleSet(Control control) => GetControlState != null ? (bool)GetControlState.Invoke(control, new object[] { 0x2 }) : control.Visible;

        /// <summary>Widens buttons whose text got cut off (usually by a translation) when there's free space next to them</summary>
        private static void FitButtons(Control Root)
        {
            foreach (Control Child in Root.Controls)
            {
                FitButtons(Child);

                if (!(Child is Button button) || !IsVisibleSet(button) || string.IsNullOrEmpty(button.Text) || button.Parent is TableLayoutPanel || button.Dock != DockStyle.None) continue;

                ButtonState State = GetState(button);
                int Reserved = button is MenuButton menuButton && menuButton.Menu != null ? (int)(18 * Program.Scale) : 0;
                int IconWidth = State.Picture != null || State.Icon != null || button.Image != null ? IconSize + (int)(6 * Program.Scale) : 0;
                int Needed = TextRenderer.MeasureText(button.Text, button.Font).Width + IconWidth + Reserved + (int)(20 * Program.Scale);

                if (Needed <= button.Width) continue;

                bool GrowLeft = (button.Anchor & AnchorStyles.Right) != 0 && (button.Anchor & AnchorStyles.Left) == 0;
                Rectangle Bounds = GrowLeft ? Rectangle.FromLTRB(button.Right - Needed, button.Top, button.Right, button.Bottom) : new Rectangle(button.Left, button.Top, Needed, button.Height);

                bool Fits = Bounds.Left >= 0 && Bounds.Right <= button.Parent.ClientSize.Width &&
                    !button.Parent.Controls.Cast<Control>().Any(Other => Other != button && IsVisibleSet(Other) && Other.Bounds.IntersectsWith(Bounds));

                if (Fits || button.Parent is FlowLayoutPanel) button.Bounds = Bounds;
            }
        }

        public static GraphicsPath RoundedRect(RectangleF Bounds, float Radius)
        {
            GraphicsPath Path = new GraphicsPath();
            float D = Radius * 2;

            if (Radius <= 0)
            {
                Path.AddRectangle(Bounds);
                return Path;
            }

            Path.AddArc(Bounds.X, Bounds.Y, D, D, 180, 90);
            Path.AddArc(Bounds.Right - D, Bounds.Y, D, D, 270, 90);
            Path.AddArc(Bounds.Right - D, Bounds.Bottom - D, D, D, 0, 90);
            Path.AddArc(Bounds.X, Bounds.Bottom - D, D, D, 90, 90);
            Path.CloseFigure();

            return Path;
        }

        #endregion

        #region Lists

        private static void StyleList(ObjectListView View)
        {
            Color Back = View.BackColor;
            Color Header = Shift(Back, 0.035f);

            View.BorderStyle = BorderStyle.None;
            View.GridLines = false;
            View.FullRowSelect = true;
            View.RowHeight = (int)(24 * Program.Scale);
            View.HeaderUsesThemes = false;
            View.HeaderMinimumHeight = (int)(26 * Program.Scale);

            HeaderFormatStyle Style = new HeaderFormatStyle();
            Style.Normal = new HeaderStateStyle { BackColor = Header, ForeColor = Mix(View.ForeColor, Back, 0.35f), FrameColor = Shift(Header, 0.06f), FrameWidth = 1f };
            Style.Hot = new HeaderStateStyle { BackColor = Shift(Header, 0.05f), ForeColor = View.ForeColor, FrameColor = Shift(Header, 0.06f), FrameWidth = 1f };
            Style.Pressed = new HeaderStateStyle { BackColor = Shift(Header, 0.1f), ForeColor = View.ForeColor, FrameColor = Shift(Header, 0.06f), FrameWidth = 1f };
            View.HeaderFormatStyle = Style;

            View.SelectedBackColor = Mix(Accent, Back, 0.25f);
            View.SelectedForeColor = ContrastText(View.SelectedBackColor);
            View.UnfocusedSelectedBackColor = Mix(Accent, Back, 0.6f);
            View.UnfocusedSelectedForeColor = View.ForeColor;

            if (View.Columns.Count > 0 && View.GetColumn(View.Columns.Count - 1) is OLVColumn Last && !View.AllColumns.Any(C => C.FillsFreeSpace))
                Last.FillsFreeSpace = true;

            void Theme() => SetWindowTheme(View.Handle, IsDark ? "DarkMode_ItemsView" : "ItemsView", null);

            if (View.IsHandleCreated) Theme(); else View.HandleCreated += (s, e) => Theme();

            if (!ListPainters.TryGetValue(View, out _))
                ListPainters.Add(View, new ListPainter(View));
        }

        private static readonly ConditionalWeakTable<ObjectListView, ListPainter> ListPainters = new ConditionalWeakTable<ObjectListView, ListPainter>();

        /// <summary>
        /// Draws the group headers & the empty space of a list itself, the dark list theme draws column lines there otherwise
        /// </summary>
        private class ListPainter : NativeWindow
        {
            private const int WM_REFLECT_NOTIFY = 0x204E, NM_CUSTOMDRAW = -12, LVCDI_GROUP = 1;
            private const int CDDS_PREPAINT = 1, CDDS_POSTPAINT = 2, CDDS_ITEMPREPAINT = 0x10001;
            private const int CDRF_SKIPDEFAULT = 4, CDRF_NOTIFYPOSTPAINT = 0x10;

            [StructLayout(LayoutKind.Sequential)] private struct RECT { public int Left, Top, Right, Bottom; }
            [StructLayout(LayoutKind.Sequential)] private struct NMHDR { public IntPtr hwndFrom; public IntPtr idFrom; public int code; }
            [StructLayout(LayoutKind.Sequential)] private struct NMCUSTOMDRAW { public NMHDR hdr; public int dwDrawStage; public IntPtr hdc; public RECT rc; public IntPtr dwItemSpec; public uint uItemState; public IntPtr lItemlParam; }
            [StructLayout(LayoutKind.Sequential)] private struct NMLVCUSTOMDRAW { public NMCUSTOMDRAW nmcd; public int clrText, clrTextBk, iSubItem, dwItemType, clrFace, iIconEffect, iIconPhase, iPartId, iStateId; public RECT rcText; public uint uAlign; }

            private readonly ObjectListView View;

            public ListPainter(ObjectListView View)
            {
                this.View = View;

                if (View.IsHandleCreated) AssignHandle(View.Handle);

                View.HandleCreated += (s, e) => AssignHandle(View.Handle);
                View.HandleDestroyed += (s, e) => ReleaseHandle();
            }

            protected override void WndProc(ref Message m)
            {
                if (m.Msg == WM_REFLECT_NOTIFY && m.LParam != IntPtr.Zero && Marshal.PtrToStructure<NMHDR>(m.LParam).code == NM_CUSTOMDRAW)
                {
                    try
                    {
                        var Draw = Marshal.PtrToStructure<NMLVCUSTOMDRAW>(m.LParam);
                        int Stage = Draw.nmcd.dwDrawStage;

                        if (Draw.dwItemType == LVCDI_GROUP && (Stage == CDDS_PREPAINT || Stage == CDDS_ITEMPREPAINT))
                        {
                            DrawGroup(Draw);
                            m.Result = (IntPtr)CDRF_SKIPDEFAULT;
                            return;
                        }

                        if (Draw.dwItemType != LVCDI_GROUP && Stage == CDDS_PREPAINT)
                        {
                            base.WndProc(ref m);
                            m.Result = (IntPtr)((long)m.Result | CDRF_NOTIFYPOSTPAINT);
                            return;
                        }

                        if (Draw.dwItemType != LVCDI_GROUP && Stage == CDDS_POSTPAINT)
                            FillEmptySpace(Draw.nmcd.hdc);
                    }
                    catch (Exception x) { Program.Logger.Error($"ListPainter error: {x.Message}"); }
                }

                base.WndProc(ref m);
            }

            private void DrawGroup(NMLVCUSTOMDRAW Draw)
            {
                Rectangle Bounds = Rectangle.FromLTRB(Draw.nmcd.rc.Left, Draw.rcText.Top, Draw.nmcd.rc.Right, Draw.rcText.Bottom); // rc is the whole group, rcText is the header row
                GetGroupInfo((int)(long)Draw.nmcd.dwItemSpec, out string Header, out bool Collapsed);

                using (Graphics g = Graphics.FromHdc(Draw.nmcd.hdc))
                using (var Back = new SolidBrush(View.BackColor))
                using (var HeaderFont = new Font(View.Font, FontStyle.Bold))
                {
                    g.FillRectangle(Back, Rectangle.FromLTRB(Draw.nmcd.rc.Left, Draw.nmcd.rc.Top, Draw.nmcd.rc.Right, Draw.nmcd.rc.Bottom)); // items are drawn on top afterwards

                    int X = Bounds.Left + (int)(8 * Program.Scale);
                    var Flags = TextFormatFlags.VerticalCenter | TextFormatFlags.Left | TextFormatFlags.SingleLine | TextFormatFlags.EndEllipsis | TextFormatFlags.NoPrefix;
                    Size TextSize = TextRenderer.MeasureText(g, Header, HeaderFont, Bounds.Size, Flags);

                    TextRenderer.DrawText(g, Header, HeaderFont, new Rectangle(X, Bounds.Top, Bounds.Width - X - 40, Bounds.Height), Mix(Accent, View.ForeColor, 0.35f), Flags);

                    int CenterY = Bounds.Top + Bounds.Height / 2;
                    int Chevron = Bounds.Right - (int)(18 * Program.Scale);

                    using (var Line = new Pen(Shift(View.BackColor, 0.1f)))
                        g.DrawLine(Line, X + TextSize.Width + 10, CenterY, Chevron - 12, CenterY);

                    g.SmoothingMode = SmoothingMode.AntiAlias;

                    float C = 3.5f * Program.Scale;

                    using (var Arrow = new Pen(Mix(View.ForeColor, View.BackColor, 0.45f), 1.6f))
                        if (Collapsed)
                            g.DrawLines(Arrow, new[] { new PointF(Chevron - C / 2, CenterY - C), new PointF(Chevron + C / 2, CenterY), new PointF(Chevron - C / 2, CenterY + C) });
                        else
                            g.DrawLines(Arrow, new[] { new PointF(Chevron - C, CenterY - C / 2), new PointF(Chevron, CenterY + C / 2), new PointF(Chevron + C, CenterY - C / 2) });
                }
            }

            [StructLayout(LayoutKind.Sequential)]
            private struct LVGROUP
            {
                public uint cbSize, mask;
                public IntPtr pszHeader;
                public int cchHeader;
                public IntPtr pszFooter;
                public int cchFooter, iGroupId;
                public uint stateMask, state, uAlign;
                public IntPtr pszSubtitle;
                public uint cchSubtitle;
                public IntPtr pszTask;
                public uint cchTask;
                public IntPtr pszDescriptionTop;
                public uint cchDescriptionTop;
                public IntPtr pszDescriptionBottom;
                public uint cchDescriptionBottom;
                public int iTitleImage, iExtendedImage, iFirstItem;
                public uint cItems;
                public IntPtr pszSubsetTitle;
                public uint cchSubsetTitle;
            }

            [DllImport("user32.dll")] private static extern IntPtr SendMessage(IntPtr hWnd, int Msg, IntPtr wParam, ref LVGROUP lParam);

            /// <summary>Reads the group header straight from the list view, the ids change whenever the list is regrouped/filtered</summary>
            private void GetGroupInfo(int GroupId, out string Header, out bool Collapsed)
            {
                const int LVM_GETGROUPINFO = 0x1000 + 149, LVGF_HEADER = 1, LVGF_STATE = 4, LVGS_COLLAPSED = 1, Length = 260;

                IntPtr Buffer = Marshal.AllocHGlobal(Length * 2);

                try
                {
                    var Group = new LVGROUP { cbSize = (uint)Marshal.SizeOf<LVGROUP>(), mask = LVGF_HEADER | LVGF_STATE, stateMask = LVGS_COLLAPSED, pszHeader = Buffer, cchHeader = Length };

                    SendMessage(View.Handle, LVM_GETGROUPINFO, (IntPtr)GroupId, ref Group);

                    Header = Marshal.PtrToStringUni(Buffer) ?? string.Empty;
                    Collapsed = (Group.state & LVGS_COLLAPSED) != 0;
                }
                finally { Marshal.FreeHGlobal(Buffer); }
            }

            private void FillEmptySpace(IntPtr Hdc)
            {
                int Bottom = View.HeaderStyle != ColumnHeaderStyle.None && View.HeaderControl != null ? View.HeaderControl.ClientRectangle.Bottom : 0;

                foreach (ListViewItem Item in View.Items)
                {
                    Rectangle Bounds = Item.Bounds;

                    if (Bounds.Height > 0 && Bounds.Bottom > Bottom) Bottom = Bounds.Bottom;
                }

                if (View.ShowGroups && View.OLVGroups != null && View.OLVGroups.Count > 0)
                    Bottom += 1;

                Rectangle Client = View.ClientRectangle;

                using (Graphics g = Graphics.FromHdc(Hdc))
                using (var Back = new SolidBrush(View.BackColor))
                {
                    foreach (ListViewItem Item in View.Items) // the 1px gaps between rows
                    {
                        Rectangle Bounds = Item.Bounds;

                        if (Bounds.Height > 0 && Bounds.Bottom < Client.Bottom && !Item.Selected)
                            g.FillRectangle(Back, new Rectangle(0, Bounds.Bottom - 1, Client.Width, 2));
                    }

                    if (Bottom < Client.Bottom)
                        g.FillRectangle(Back, new Rectangle(0, Math.Max(Bottom, 0), Client.Width, Client.Bottom - Math.Max(Bottom, 0)));
                }
            }
        }

        #endregion

        #region Menus

        private static readonly ConditionalWeakTable<ContextMenuStrip, object> StyledMenus = new ConditionalWeakTable<ContextMenuStrip, object>();

        public static void StyleMenu(ContextMenuStrip Strip)
        {
            if (!StyledMenus.TryGetValue(Strip, out _))
            {
                StyledMenus.Add(Strip, null);

                Strip.Opening += (s, e) => StyleMenu(Strip); // theme might have changed
                Strip.HandleCreated += (s, e) =>
                {
                    int Round = 3; // DWMWCP_ROUNDSMALL
                    DwmSetWindowAttribute(Strip.Handle, DWMWA_WINDOW_CORNER_PREFERENCE, ref Round, sizeof(int));
                };
            }

            Strip.Renderer = new MenuRenderer();
            Strip.Padding = new Padding(2, 4, 2, 4);
            Strip.ShowImageMargin = true;

            StyleMenuItems(Strip.Items);
        }

        private static void StyleMenuItems(ToolStripItemCollection Items)
        {
            foreach (ToolStripItem Item in Items)
            {
                Item.ForeColor = ThemeEditor.FormsForeground;

                if (Item is ToolStripMenuItem MenuItem)
                {
                    MenuItem.Padding = new Padding(4, (int)(3 * Program.Scale), 4, (int)(3 * Program.Scale));

                    if (MenuIcons.TryGetValue(MenuItem.Name ?? string.Empty, out string Icon))
                        MenuItem.Image = ModernUI.Icon(Icon, IconSize, Muted);

                    if (MenuItem.HasDropDownItems)
                    {
                        MenuItem.DropDown.Renderer = new MenuRenderer();
                        StyleMenuItems(MenuItem.DropDownItems);
                    }
                }
            }
        }

        private class MenuColors : ProfessionalColorTable
        {
            private static Color Back => Shift(ThemeEditor.FormsBackground, 0.05f);

            public override Color ToolStripDropDownBackground => Back;
            public override Color ImageMarginGradientBegin => Back;
            public override Color ImageMarginGradientMiddle => Back;
            public override Color ImageMarginGradientEnd => Back;
            public override Color MenuBorder => Border;
            public override Color MenuItemBorder => Color.Transparent;
            public override Color MenuItemSelected => Shift(Back, 0.07f);
            public override Color SeparatorDark => Border;
            public override Color SeparatorLight => Border;
        }

        public class MenuRenderer : ToolStripProfessionalRenderer
        {
            public MenuRenderer() : base(new MenuColors()) => RoundedEdges = false;

            protected override void OnRenderItemText(ToolStripItemTextRenderEventArgs e)
            {
                e.TextColor = e.Item.Enabled ? ThemeEditor.FormsForeground : Muted;
                base.OnRenderItemText(e);
            }

            protected override void OnRenderArrow(ToolStripArrowRenderEventArgs e)
            {
                e.ArrowColor = Muted;
                base.OnRenderArrow(e);
            }

            protected override void OnRenderMenuItemBackground(ToolStripItemRenderEventArgs e)
            {
                if (!e.Item.Selected || !e.Item.Enabled) return;

                e.Graphics.SmoothingMode = SmoothingMode.AntiAlias;

                using (var Brush = new SolidBrush(ColorTable.MenuItemSelected))
                using (var Path = RoundedRect(new RectangleF(4, 1, e.Item.Width - 7, e.Item.Height - 2), 4 * Program.Scale))
                    e.Graphics.FillPath(Brush, Path);
            }

            protected override void OnRenderItemCheck(ToolStripItemImageRenderEventArgs e)
            {
                e.Graphics.SmoothingMode = SmoothingMode.AntiAlias;

                Rectangle Box = e.ImageRectangle;
                Box.Inflate(2, 2);

                using (var Brush = new SolidBrush(Mix(Accent, ThemeEditor.FormsBackground, 0.4f)))
                using (var Path = RoundedRect(Box, 3 * Program.Scale))
                    e.Graphics.FillPath(Brush, Path);

                if (e.Image != null)
                    e.Graphics.DrawImage(e.Image, e.ImageRectangle);
            }
        }

        #endregion
    }

    /// <summary>A panel drawn as a rounded "card"</summary>
    public class CardPanel : Panel
    {
        public Func<Color> Fill;
        public float Radius = 10f;

        public CardPanel()
        {
            DoubleBuffered = true;
            ResizeRedraw = true;
        }

        protected override void OnPaintBackground(PaintEventArgs e)
        {
            Color Outside = Parent?.BackColor ?? ThemeEditor.FormsBackground;

            e.Graphics.Clear(Outside);
            e.Graphics.SmoothingMode = SmoothingMode.AntiAlias;

            using (var Brush = new SolidBrush(BackColor))
            using (var Path = ModernUI.RoundedRect(new RectangleF(0, 0, Width - 1, Height - 1), Radius * Program.Scale))
                e.Graphics.FillPath(Brush, Path);
        }
    }
}
