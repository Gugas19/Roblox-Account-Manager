using RBX_Alt_Manager.Classes;
using RBX_Alt_Manager.Forms;
using System;
using System.Diagnostics;
using System.Drawing;
using System.Linq;
using System.Net;
using System.Threading.Tasks;
using System.Windows.Forms;
using MessageBox = RBX_Alt_Manager.Classes.LocalizedMessageBox;

namespace RBX_Alt_Manager
{
    /// <summary>
    /// Modern layout of the main window, applied on top of the designer layout (before scaling)
    /// </summary>
    public partial class AccountManager
    {
        private CardPanel ListCard, SideCard;
        private Label GameSection, FollowSection, AccountSection;
        private Button LanguageButton;
        private Panel ListHeader;
        private BorderedTextBox SearchBox;
        private PictureBox SearchIcon;
        private Label CountLabel;
        private ContextMenuStrip LanguageStrip;

        private void ApplyModernLayout()
        {
            const int Margin = 16, Gap = 8;
            const int Width = 980, Height = 560;
            const int SideWidth = 336, SidePad = 16;

            SuspendLayout();

            Font = new Font(ModernUI.FontName, 9f);
            MinimumSize = new Size(900, 520);
            ClientSize = new Size(Width, Height);

            var TopLeft = AnchorStyles.Top | AnchorStyles.Left;
            var TopRight = AnchorStyles.Top | AnchorStyles.Right;

            // Top bar: account actions on the left, app actions on the right
            Place(Add, Margin, Margin, 156, 34, TopLeft);
            Place(Remove, Margin + 156 + Gap, Margin, 120, 34, TopLeft);
            Place(OpenBrowser, Margin + 156 + 120 + Gap * 2, Margin, 170, 34, TopLeft);
            Place(HideUsernamesCheckbox, Margin + 156 + 120 + 170 + Gap * 3 + 4, Margin + 8, 150, 20, TopLeft);
            HideUsernamesCheckbox.AutoSize = true;

            DLChromiumLabel.Location = new Point(Margin, Margin);
            DLChromiumLabel.Anchor = TopLeft;
            Place(DownloadProgressBar, Margin, Margin + 22, 250, 8, TopLeft);

            Place(ConfigButton, Width - Margin - 34, Margin, 34, 34, TopRight);
            Place(EditTheme, Width - Margin - 34 * 2 - Gap, Margin, 34, 34, TopRight);
            EditTheme.Text = string.Empty;
            SaveTooltip.SetToolTip(EditTheme, "Edit Theme");
            SaveTooltip.SetToolTip(ConfigButton, "Settings");

            LanguageButton = new Button { Name = "LanguageButton", TabStop = false };
            Controls.Add(LanguageButton);
            Place(LanguageButton, Width - Margin - 34 * 2 - Gap * 2 - 74, Margin, 74, 34, TopRight);
            SaveTooltip.SetToolTip(LanguageButton, "Language");
            LanguageButton.Click += (s, e) => LanguageStrip.Show(LanguageButton, new Point(0, LanguageButton.Height + 2));

            Place(JoinDiscord, Width - Margin - 34 * 2 - Gap * 3 - 74 - 34, Margin, 34, 34, TopRight);
            SaveTooltip.SetToolTip(JoinDiscord, "Join Discord");

            BuildLanguageMenu();

            // Accounts list card
            int ContentTop = Margin + 34 + 12;

            ListCard = new CardPanel { Fill = () => ThemeEditor.AccountBackground, Padding = new Padding(8, 6, 8, 8) };
            Controls.Add(ListCard);
            Place(ListCard, Margin, ContentTop, Width - Margin * 2 - SideWidth - 12, Height - ContentTop - Margin, AnchorStyles.Top | AnchorStyles.Bottom | AnchorStyles.Left | AnchorStyles.Right);

            AccountsView.Parent = ListCard;
            AccountsView.Dock = DockStyle.Fill;
            AccountsView.Font = new Font(ModernUI.FontName, 9.5f);
            Description.FillsFreeSpace = true;

            // Search & count above the list
            ListHeader = new Panel { Dock = DockStyle.Top, Height = 40, BackColor = Color.Transparent };
            ListCard.Controls.Add(ListHeader);
            AccountsView.BringToFront();

            SearchIcon = new PictureBox { Bounds = new Rectangle(6, 9, 20, 20), SizeMode = PictureBoxSizeMode.CenterImage, Image = new Bitmap(ModernUI.Icon("search", 18, Color.White)), BackColor = Color.Transparent };
            SearchBox = new BorderedTextBox { Name = "SearchBox", Bounds = new Rectangle(30, 7, 240, 23), BorderStyle = BorderStyle.Fixed3D };
            CountLabel = new Label { Name = "CountLabel", AutoSize = false, TextAlign = ContentAlignment.MiddleRight, Bounds = new Rectangle(280, 7, ListCard.Width - 16 - 280, 23), Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right };
            ListHeader.Controls.AddRange(new Control[] { SearchIcon, SearchBox, CountLabel });
            ModernUI.SetPlaceholder(SearchBox, "Search accounts...");
            SaveTooltip.SetToolTip(SearchBox, "Search by username, alias, description or group (Ctrl+F)");

            SearchBox.TextChanged += (s, e) => ApplySearch();
            SearchBox.KeyDown += (s, e) => { if (e.KeyCode == Keys.Escape) { SearchBox.Clear(); AccountsView.Focus(); e.SuppressKeyPress = true; } };
            AccountsView.ItemsChanged += (s, e) => { UpdateAccountCount(); UpdateAccountButtons(); };
            AccountsView.SelectionChanged += (s, e) => { UpdateAccountCount(); UpdateAccountButtons(); };
            AccountsView.KeyDown += (s, e) => { if (e.KeyCode == Keys.Delete && AccountsView.SelectedObjects.Count > 0) { Remove.PerformClick(); e.Handled = true; } };

            KeyPreview = true;
            KeyDown += (s, e) =>
            {
                if (e.Control && e.KeyCode == Keys.F) { SearchBox.Focus(); SearchBox.SelectAll(); e.SuppressKeyPress = true; }
            };

            // Side card: game, follow & account sections
            SideCard = new CardPanel();
            Controls.Add(SideCard);
            Place(SideCard, Width - Margin - SideWidth, ContentTop, SideWidth, Height - ContentTop - Margin, AnchorStyles.Top | AnchorStyles.Bottom | AnchorStyles.Right);

            int Inner = SideWidth - SidePad * 2;
            var InTop = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right;
            int Y = 14;

            GameSection = SectionLabel("GAME", SidePad, Y, Inner);
            Y += 20;

            CurrentPlace.AutoSize = false;
            Reparent(CurrentPlace, SidePad, Y, Inner, 22, InTop);
            CurrentPlace.AutoEllipsis = true;
            CurrentPlace.Font = new Font(ModernUI.FontName, 10.5f, FontStyle.Bold);
            CurrentPlace.Tag = "UseControlFont";
            Y += 30;

            int PlaceWidth = 116, SaveWidth = 30;
            int JobX = SidePad + PlaceWidth + Gap, JobWidth = Inner - PlaceWidth - Gap * 2 - SaveWidth;

            Reparent(LabelPlaceID, SidePad, Y, 80, 16, AnchorStyles.Top | AnchorStyles.Left);
            Reparent(HistoryIcon, SidePad + PlaceWidth - 18, Y - 1, 18, 18, AnchorStyles.Top | AnchorStyles.Left);
            Reparent(LabelJobID, JobX, Y, 80, 16, AnchorStyles.Top | AnchorStyles.Left);
            Reparent(ShuffleIcon, JobX + JobWidth - 18, Y - 1, 18, 18, AnchorStyles.Top | AnchorStyles.Right);
            Y += 20;

            Reparent(PlaceID, SidePad, Y, PlaceWidth, 23, AnchorStyles.Top | AnchorStyles.Left);
            Reparent(JobID, JobX, Y, JobWidth, 23, InTop);
            Reparent(SaveToAccount, SidePad + Inner - SaveWidth, Y - 1, SaveWidth, 27, AnchorStyles.Top | AnchorStyles.Right);
            SaveToAccount.Image = null;
            HistoryIcon.Image = new Bitmap(ModernUI.Icon("history", 18, Color.White)); // copies, the theme recolors these in place
            ShuffleIcon.Image = new Bitmap(ModernUI.Icon("shuffle", 18, Color.White));
            HistoryIcon.SizeMode = ShuffleIcon.SizeMode = PictureBoxSizeMode.CenterImage;
            SaveTooltip.SetToolTip(ShuffleIcon, "Shuffle Job ID");
            SaveTooltip.SetToolTip(HistoryIcon, "Recent games");
            Y += 36;

            Reparent(JoinServer, SidePad, Y, Inner, 38, InTop);
            Y += 54;

            int ActionWidth = 122, FieldWidth = Inner - ActionWidth - Gap;

            FollowSection = SectionLabel("FOLLOW PLAYER", SidePad, Y, Inner);
            Y += 20;
            LabelUserID.Visible = false;
            Reparent(UserID, SidePad, Y + 2, FieldWidth, 23, InTop);
            Reparent(Follow, SidePad + FieldWidth + Gap, Y, ActionWidth, 28, AnchorStyles.Top | AnchorStyles.Right);
            ModernUI.SetPlaceholder(UserID, "Username");
            Y += 44;

            AccountSection = SectionLabel("SELECTED ACCOUNT", SidePad, Y, Inner);
            Y += 20;
            Alias.Text = string.Empty;
            Reparent(Alias, SidePad, Y + 2, FieldWidth, 23, InTop);
            Reparent(SetAlias, SidePad + FieldWidth + Gap, Y, ActionWidth, 28, AnchorStyles.Top | AnchorStyles.Right);
            ModernUI.SetPlaceholder(Alias, "Alias");
            ModernUI.SetPlaceholder(PlaceID, "Place ID");
            ModernUI.SetPlaceholder(JobID, "Job ID or VIP link");
            Y += 38;

            int SideHeight = SideCard.Height;
            int BottomRow = SideHeight - 14 - 34;
            int DescriptionButtonY = BottomRow - Gap - 30;

            DescriptionBox.Text = string.Empty;
            Reparent(DescriptionBox, SidePad, Y, Inner, DescriptionButtonY - Gap - Y, AnchorStyles.Top | AnchorStyles.Bottom | AnchorStyles.Left | AnchorStyles.Right);
            Reparent(SetDescription, SidePad, DescriptionButtonY, Inner, 30, AnchorStyles.Bottom | AnchorStyles.Left | AnchorStyles.Right);

            int Half = (Inner - Gap) / 2;
            Reparent(ServerList, SidePad, BottomRow, Half, 34, AnchorStyles.Bottom | AnchorStyles.Left);
            Reparent(BrowserButton, SidePad + Half + Gap, BottomRow, Inner - Half - Gap, 34, AnchorStyles.Bottom | AnchorStyles.Right);

            // Encryption / password screens stay centered when the window is resized
            EncryptionSelectionPanel.Anchor = AnchorStyles.None;
            PasswordSelectionPanel.Anchor = AnchorStyles.None;
            PasswordLayoutPanel.Anchor = AnchorStyles.None;
            PasswordPanel.Size = ClientSize;
            CenterIn(EncryptionSelectionPanel, PasswordPanel);
            CenterIn(PasswordSelectionPanel, PasswordPanel);
            CenterIn(PasswordLayoutPanel, PasswordPanel);
            DefaultEncryptionButton.Height = PasswordEncryptionButton.Height = 46;

            PasswordPanel.BringToFront();

            ResumeLayout(false);

            Lang.LanguageChanged += (s, e) =>
            {
                UpdateLanguageButton();
                ApplyTheme();
                UpdateAccountCount();
            };
        }

        private void ApplySearch()
        {
            string Query = SearchBox.Text.Trim();

            if (string.IsNullOrEmpty(Query))
            {
                AccountsView.ModelFilter = null;
                AccountsView.UseFiltering = false;
            }
            else
            {
                AccountsView.UseFiltering = true;
                AccountsView.ModelFilter = new BrightIdeasSoftware.ModelFilter(Model => Model is Account account && (
                    Matches(account.Username, Query) || Matches(account.Alias, Query) || Matches(account.Description, Query) || Matches(account.Group, Query)));
            }

            if (AccountsView.ShowGroups) AccountsView.BuildGroups(); // otherwise filtered accounts end up in a single "Default" group

            UpdateAccountCount();
        }

        /// <summary>Buttons that act on the selected account(s) are only enabled while something is selected</summary>
        public void UpdateAccountButtons()
        {
            int Selected = AccountsView.SelectedObjects.Count;

            foreach (Button button in new Button[] { Remove, OpenBrowser, JoinServer, Follow, SetAlias, SetDescription, SaveToAccount })
                button.Enabled = Selected > 0;

            BrowserButton.Enabled = Selected == 1;
        }

        private static bool Matches(string Value, string Query) => !string.IsNullOrEmpty(Value) && Value.IndexOf(Query, StringComparison.OrdinalIgnoreCase) >= 0;

        /// <summary>Count label, empty list message & styling details that depend on the theme/language</summary>
        public void UpdateAccountCount()
        {
            if (CountLabel == null || AccountsList == null) return;

            int Total = AccountsList.Count, Shown = AccountsView.GetItemCount(), Selected = AccountsView.SelectedObjects.Count;
            string Text = Shown == Total ? Lang.F(Total == 1 ? "{0} account" : "{0} accounts", Total) : Lang.F("{0} of {1} accounts", Shown, Total);

            if (Selected > 1) Text += " · " + Lang.F("{0} selected", Selected);

            CountLabel.Text = Text;
            CountLabel.ForeColor = ModernUI.Muted;
            SearchIcon.Image = new Bitmap(ModernUI.Icon("search", 18, ModernUI.Muted));

            AccountsView.EmptyListMsg = string.IsNullOrEmpty(SearchBox.Text.Trim()) ? Lang.T("No accounts yet, click \"Add Account\" to add one") : Lang.T("No accounts match your search");
            AccountsView.EmptyListMsgFont = new Font(ModernUI.FontName, 10f * Program.Scale);

            if (AccountsView.EmptyListMsgOverlay is BrightIdeasSoftware.TextOverlay Overlay)
            {
                Overlay.TextColor = ModernUI.Muted;
                Overlay.BackColor = AccountsView.BackColor;
                Overlay.BorderColor = AccountsView.BackColor;
                Overlay.BorderWidth = 0;
                Overlay.Alignment = ContentAlignment.MiddleCenter;
            }
        }

        #region Settings that apply right away (no restart needed)

        private System.Timers.Timer PresenceTimer;

        /// <summary>Shows or hides the developer menu items</summary>
        public void ApplyDeveloperMode()
        {
            bool DevMode = Developer.Get<bool>("DevMode");

            foreach (ToolStripItem Item in new ToolStripItem[] { viewFieldsToolStripMenuItem, getAuthenticationTicketToolStripMenuItem, copyRbxplayerLinkToolStripMenuItem, copySecurityTokenToolStripMenuItem, copyAppLinkToolStripMenuItem })
                Item.Visible = DevMode;
        }

        public void ApplyAutoCookieRefresh()
        {
            bool Enabled = General.Get<bool>("AutoCookieRefresh");

            if (AutoCookieRefresh == null && Enabled)
            {
                AutoCookieRefresh = new System.Timers.Timer(60000 * 5);
                AutoCookieRefresh.Elapsed += RefreshOldCookies;
            }

            if (AutoCookieRefresh != null)
                AutoCookieRefresh.Enabled = Enabled;
        }

        private async void RefreshOldCookies(object sender, System.Timers.ElapsedEventArgs e)
        {
            foreach (var Account in AccountsList.ToList())
            {
                if (Account.GetField("NoCookieRefresh") != "true" && (DateTime.Now - Account.LastUse).TotalDays > 20 && (DateTime.Now - Account.LastAttemptedRefresh).TotalDays >= 7)
                {
                    Program.Logger.Info($"Attempting to refresh {Account.Username} | Last Use: {Account.LastUse}");

                    Account.LastAttemptedRefresh = DateTime.Now;
                    Account.LogOutOfOtherSessions(true);

                    await Task.Delay(5000);
                }
            }
        }

        /// <summary>How often the online/in game dots are refreshed, from the "PresenceUpdateRate" setting (minutes)</summary>
        public void ApplyPresenceRate()
        {
            int Minutes = Math.Max(1, General.Exists("PresenceUpdateRate") ? General.Get<int>("PresenceUpdateRate") : 2);

            if (PresenceTimer == null)
            {
                PresenceTimer = new System.Timers.Timer();
                PresenceTimer.Elapsed += (s, e) => AccountsView.InvokeIfRequired(async () => await UpdatePresence());
            }

            PresenceTimer.Interval = Minutes * 60000;
            PresenceTimer.Enabled = true;
        }

        /// <summary>Redraws the account list, used when the aging alert or presence settings change</summary>
        public void RefreshAccountsView()
        {
            AccountsView.Invalidate();

            if (General.Get<bool>("ShowPresence"))
                _ = UpdatePresence();
        }

        /// <summary>(Re)starts the local web server with the current settings, or stops it when it's disabled</summary>
        public void RestartWebServer()
        {
            try { AltManagerWS?.Stop(); } catch { }

            AltManagerWS = null;

            if (!Developer.Get<bool>("EnableWebServer")) return;

            string Port = WebServer.Exists("WebServerPort") ? WebServer.Get("WebServerPort") : "7963";
            bool External = WebServer.Get<bool>("AllowExternalConnections");
            string[] LocalOnly = { $"http://localhost:{Port}/" };
            string[] Everywhere = { $"http://localhost:{Port}/", $"http://*:{Port}/" };

            try { StartWebServer(External ? Everywhere : LocalOnly); }
            catch (HttpListenerException x) when (External && x.ErrorCode == 5) // listening on every address needs a one time permission from an administrator
            {
                if (Utilities.YesNoPrompt("Roblox Account Manager", "Allowing external connections needs administrator permission once", "Windows will ask for permission to let the web server accept connections from other devices. Continue?", false) && AllowWebServerAddress(Port))
                    try
                    {
                        StartWebServer(Everywhere);

                        return;
                    }
                    catch (Exception y) { Program.Logger.Error($"Web server still can't listen on every address: {y.Message}"); }

                try { StartWebServer(LocalOnly); } catch (Exception y) { MessageBox.Show($"Failed to start webserver!\n\n{y.Message}", "Roblox Account Manager", MessageBoxButtons.OK, MessageBoxIcon.Error); return; }

                MessageBox.Show("External connections aren't allowed yet, the web server only accepts connections from this PC.", "Roblox Account Manager", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            }
            catch (Exception x) { MessageBox.Show($"Failed to start webserver!\n\n{x.Message}", "Roblox Account Manager", MessageBoxButtons.OK, MessageBoxIcon.Error); }
        }

        private void StartWebServer(string[] Prefixes)
        {
            AltManagerWS = new WebServer(SendResponse, Prefixes);
            AltManagerWS.Run();
        }

        /// <summary>Registers http://*:port/ for everyone (one UAC prompt), instead of having to run the whole app as administrator</summary>
        private static bool AllowWebServerAddress(string Port)
        {
            try
            {
                using (Process Netsh = Process.Start(new ProcessStartInfo("netsh", $"http add urlacl url=http://*:{Port}/ sddl=D:(A;;GX;;;WD)") { Verb = "runas", UseShellExecute = true, WindowStyle = ProcessWindowStyle.Hidden }))
                    Netsh.WaitForExit();

                return true;
            }
            catch (Exception x) // the user declined the prompt
            {
                Program.Logger.Warn($"URL reservation for the web server wasn't made: {x.Message}");

                return false;
            }
        }

        #endregion

        private void Place(Control control, int X, int Y, int W, int H, AnchorStyles Anchor)
        {
            control.Anchor = Anchor;
            control.Bounds = new Rectangle(X, Y, W, H);
        }

        private void Reparent(Control control, int X, int Y, int W, int H, AnchorStyles Anchor)
        {
            control.Parent = SideCard;
            Place(control, X, Y, W, H, Anchor);
        }

        private static void CenterIn(Control control, Control Parent) =>
            control.Location = new Point((Parent.Width - control.Width) / 2, (Parent.Height - control.Height) / 2);

        private Label SectionLabel(string Text, int X, int Y, int W)
        {
            Label label = new Label
            {
                Text = Text,
                AutoSize = false,
                Bounds = new Rectangle(X, Y, W, 16),
                Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right,
                Font = new Font(ModernUI.FontName, 7.75f, FontStyle.Bold),
                Tag = "UseControlFont",
                Name = "Section" + Text.Replace(" ", "")
            };

            SideCard.Controls.Add(label);

            return label;
        }

        private void BuildLanguageMenu()
        {
            LanguageStrip = new ContextMenuStrip(components);

            foreach (Language language in Lang.Languages)
            {
                var Item = new ToolStripMenuItem(language.Name) { Tag = language, Name = "Language_" + language.Code };

                Item.Click += (s, e) => Lang.SetLanguage(language.Code);
                LanguageStrip.Items.Add(Item);
            }

            LanguageStrip.Opening += (s, e) => UpdateLanguageMenu();
            ModernUI.StyleMenu(LanguageStrip);
            UpdateLanguageMenu();
            UpdateLanguageButton();
        }

        private void UpdateLanguageMenu()
        {
            foreach (ToolStripMenuItem Item in LanguageStrip.Items)
            {
                Language language = (Language)Item.Tag;

                Item.Image = ModernUI.Flag(language.Flag, (int)(14 * Program.Scale));
                Item.Font = new Font(LanguageStrip.Font, language == Lang.Current ? FontStyle.Bold : FontStyle.Regular);
                Item.ForeColor = ThemeEditor.FormsForeground;
            }
        }

        private void UpdateLanguageButton()
        {
            if (LanguageButton == null) return;

            ModernUI.SetImage(LanguageButton, ModernUI.Flag(Lang.Current.Flag, (int)(14 * Program.Scale)));
            LanguageButton.Text = Lang.Current.Code.Length > 2 ? Lang.Current.Code.Substring(3) : Lang.Current.Code.ToUpper();
            LanguageButton.Invalidate();
        }
    }
}
