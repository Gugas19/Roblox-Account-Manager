using Microsoft.Win32;
using System;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Text.RegularExpressions;
using System.Windows.Forms;
using MessageBox = RBX_Alt_Manager.Classes.LocalizedMessageBox;

namespace RBX_Alt_Manager.Forms
{
    public partial class SettingsForm : Form
    {
        private bool SettingsLoaded = false;
        private RegistryKey StartupKey;

        public SettingsForm()
        {
            AccountManager.SetDarkBar(Handle);

            InitializeComponent();
            AddLanguageSelector();
            ClientSize = new Size(ClientSize.Width + 90, ClientSize.Height + 40);
            MinimumSize = new Size(MinimumSize.Width + 90, MinimumSize.Height + 40);
            RSLabel.Visible = false; // "some settings require a restart", every setting applies right away now
            this.Rescale();
        }

        private ComboBox LanguageCB;
        private bool UpdatingLanguage;

        private void AddLanguageSelector()
        {
            Label LanguageLabel = new Label { Text = "Language", AutoSize = true, Margin = new Padding(3, 7, 8, 3), Name = "LanguageLabel" };

            LanguageCB = new ComboBox
            {
                Name = "LanguageCB",
                DropDownStyle = ComboBoxStyle.DropDownList,
                DrawMode = DrawMode.OwnerDrawFixed,
                ItemHeight = 22,
                Width = 200,
                Margin = new Padding(3, 3, 3, 10)
            };

            foreach (Classes.Language language in Classes.Lang.Languages)
                LanguageCB.Items.Add(language);

            LanguageCB.DrawItem += (s, e) =>
            {
                if (e.Index < 0) return;

                Classes.Language language = (Classes.Language)LanguageCB.Items[e.Index];
                bool Highlight = (e.State & DrawItemState.Selected) != 0 && (e.State & DrawItemState.ComboBoxEdit) == 0;
                Color Back = Highlight ? Classes.ModernUI.Mix(Classes.ModernUI.Accent, ThemeEditor.TextBoxesBackground, 0.35f) : ThemeEditor.TextBoxesBackground;

                using (var Brush = new SolidBrush(Back))
                    e.Graphics.FillRectangle(Brush, e.Bounds);

                Image Flag = Classes.ModernUI.Flag(language.Flag, Math.Max(10, e.Bounds.Height - 10));
                int X = e.Bounds.X + 6;

                if (Flag != null)
                {
                    e.Graphics.DrawImage(Flag, X, e.Bounds.Y + (e.Bounds.Height - Flag.Height) / 2);
                    X += Flag.Width + 8;
                }

                TextRenderer.DrawText(e.Graphics, language.Name, LanguageCB.Font, new Rectangle(X, e.Bounds.Y, e.Bounds.Right - X, e.Bounds.Height), ThemeEditor.TextBoxesForeground, TextFormatFlags.VerticalCenter | TextFormatFlags.Left | TextFormatFlags.SingleLine);
            };

            LanguageCB.SelectedIndexChanged += (s, e) =>
            {
                if (UpdatingLanguage || !(LanguageCB.SelectedItem is Classes.Language language) || language == Classes.Lang.Current) return;

                Classes.Lang.SetLanguage(language.Code);
            };

            void SyncSelection()
            {
                UpdatingLanguage = true;
                LanguageCB.SelectedItem = Classes.Lang.Current;
                UpdatingLanguage = false;
            }

            SyncSelection();
            Classes.Lang.LanguageChanged += (s, e) => SyncSelection();

            SettingsLayoutPanel.Controls.Add(LanguageLabel);
            SettingsLayoutPanel.Controls.Add(LanguageCB);
            SettingsLayoutPanel.Controls.SetChildIndex(LanguageLabel, 0);
            SettingsLayoutPanel.Controls.SetChildIndex(LanguageCB, 1);
            SettingsLayoutPanel.SetFlowBreak(LanguageCB, true);
        }

        private void SettingsForm_Load(object sender, EventArgs e)
        {
            AsyncJoinCB.Checked = AccountManager.General.Get<bool>("AsyncJoin");
            LaunchDelayNumber.Value = AccountManager.General.Get<decimal>("AccountJoinDelay");
            SavePasswordCB.Checked = AccountManager.General.Get<bool>("SavePasswords");
            DisableAgingAlertCB.Checked = AccountManager.General.Get<bool>("DisableAgingAlert");
            HideMRobloxCB.Checked = AccountManager.General.Get<bool>("HideRbxAlert");
            DisableImagesCB.Checked = AccountManager.General.Get<bool>("DisableImages");
            ShuffleLowestServerCB.Checked = AccountManager.General.Get<bool>("ShuffleChoosesLowestServer");
            MultiRobloxCB.Checked = AccountManager.General.Get<bool>("EnableMultiRbx");
            RegionFormatTB.Text = AccountManager.General.Get<string>("ServerRegionFormat");
            MaxRecentGamesNumber.Value = AccountManager.General.Get<int>("MaxRecentGames");

            EnableDMCB.Checked = AccountManager.Developer.Get<bool>("DevMode");
            EnableWSCB.Checked = AccountManager.Developer.Get<bool>("EnableWebServer");
            ERRPCB.Checked = AccountManager.WebServer.Get<bool>("EveryRequestRequiresPassword");
            AllowGCCB.Checked = AccountManager.WebServer.Get<bool>("AllowGetCookie");
            AllowGACB.Checked = AccountManager.WebServer.Get<bool>("AllowGetAccounts");
            AllowLACB.Checked = AccountManager.WebServer.Get<bool>("AllowLaunchAccount");
            AllowAECB.Checked = AccountManager.WebServer.Get<bool>("AllowAccountEditing");
            AllowExternalConnectionsCB.Checked = AccountManager.WebServer.Get<bool>("AllowExternalConnections");
            PasswordTextBox.Text = AccountManager.WebServer.Get("Password");
            PortNumber.Value = AccountManager.WebServer.Get<decimal>("WebServerPort");

            PresenceCB.Checked = AccountManager.General.Get<bool>("ShowPresence");
            AutoCookieRefreshCB.Checked = AccountManager.General.Get<bool>("AutoCookieRefresh");
            PresenceUpdateRateNum .Value = AccountManager.General.Get<int>("PresenceUpdateRate");
            UnlockFPSCB.Checked = AccountManager.General.Get<bool>("UnlockFPS");
            MaxFPSValue.Value = AccountManager.General.Get<int>("MaxFPSValue");

            if (AccountManager.General.Exists("CustomClientSettings") && File.Exists(AccountManager.General.Get<string>("CustomClientSettings")))
            {
                OverrideWithCustomCB.Checked = true;
                UnlockFPSCB.Enabled = false;
            }

            try { StartupKey = Registry.CurrentUser.OpenSubKey(@"SOFTWARE\Microsoft\Windows\CurrentVersion\Run", true); } catch { }

            if (StartupKey != null && StartupKey.GetValue(Application.ProductName) is string ExistingPath)
            {
                if (ExistingPath != Application.ExecutablePath) // fix the path if moved
                    StartupKey.SetValue(Application.ProductName, Application.ExecutablePath);

                StartOnPCStartup.Checked = true;
            }

            SettingsLoaded = true;

            ApplyTheme();
        }

        private void SettingsForm_FormClosing(object sender, FormClosingEventArgs e)
        {
            e.Cancel = true;
            Hide();
        }

        #region General

        private void AsyncJoinCB_CheckedChanged(object sender, EventArgs e)
        {
            LaunchDelayNumber.Enabled = !AsyncJoinCB.Checked;

            if (!SettingsLoaded) return;

            AccountManager.General.Set("AsyncJoin", AsyncJoinCB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        private void LaunchDelayNumber_ValueChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.General.Set("AccountJoinDelay", LaunchDelayNumber.Value.ToString());
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        private void MaxRecentGamesNumber_ValueChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.General.Set("MaxRecentGames", MaxRecentGamesNumber.Value.ToString());
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        private void SavePasswordCB_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.General.Set("SavePasswords", SavePasswordCB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");

            if (!SavePasswordCB.Checked && AccountManager.AccountsList.Any(account => !string.IsNullOrEmpty(account.Password))
                && Utilities.YesNoPrompt("Settings", "Remove saved passwords?", "Do you also want to remove every password that is already saved?", false))
            {
                foreach (Account account in AccountManager.AccountsList)
                    account.Password = string.Empty;

                AccountManager.SaveAccounts(true);
            }
        }

        private void DisableAgingAlertCB_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.General.Set("DisableAgingAlert", DisableAgingAlertCB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
            AccountManager.Instance.RefreshAccountsView();
        }

        private void HideMRobloxCB_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.General.Set("HideRbxAlert", HideMRobloxCB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        private void RegionFormatTB_TextChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.General.Set("ServerRegionFormat", RegionFormatTB.Text, "Visit http://ip-api.com/json to see available format options");
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        private void DisableImagesCB_CheckedChanged(object sender, EventArgs e)
        {
            AccountManager.General.Set("DisableImages", DisableImagesCB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        private void ShuffleLowestServerCB_CheckedChanged(object sender, EventArgs e)
        {
            AccountManager.General.Set("ShuffleChoosesLowestServer", ShuffleLowestServerCB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        private void MultiRobloxCB_CheckedChanged(object sender, EventArgs e)
        {
            AccountManager.General.Set("EnableMultiRbx", MultiRobloxCB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");

            if (!AccountManager.Instance.UpdateMultiRoblox())
                MessageBox.Show("Roblox is currently running, multi roblox will not work if roblox is open.", "Roblox Account Manager", MessageBoxButtons.OK, MessageBoxIcon.Warning);
        }

        private void AutoCookieRefreshCB_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.General.Set("AutoCookieRefresh", AutoCookieRefreshCB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
            AccountManager.Instance.ApplyAutoCookieRefresh();
        }

        private void StartOnPCStartup_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            if (StartOnPCStartup.Checked)
                StartupKey?.SetValue(Application.ProductName, Application.ExecutablePath);
            else
                StartupKey?.DeleteValue(Application.ProductName, false); // don't crash if it was already removed elsewhere
        }

        private void EncryptionSelectionButton_Click(object sender, EventArgs e)
        {
            if (Utilities.YesNoPrompt("Settings", "Change Encryption Method", "Are you sure you want to change how your data is encrypted?", false))
                AccountManager.Instance.ResetEncryption(true);
        }

        #endregion

        #region Developer

        private void EnableDMCB_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.Developer.Set("DevMode", EnableDMCB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
            AccountManager.Instance.ApplyDeveloperMode();
        }

        private void EnableWSCB_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.Developer.Set("EnableWebServer", EnableWSCB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
            AccountManager.Instance.RestartWebServer();
        }

        private void ERRPCB_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.WebServer.Set("EveryRequestRequiresPassword", ERRPCB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        private void AllowGCCB_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.WebServer.Set("AllowGetCookie", AllowGCCB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        private void AllowGACB_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.WebServer.Set("AllowGetAccounts", AllowGACB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        private void AllowLACB_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.WebServer.Set("AllowLaunchAccount", AllowLACB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        private void AllowAECB_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.WebServer.Set("AllowAccountEditing", AllowAECB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        private void AllowExternalConnectionsCB_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.WebServer.Set("AllowExternalConnections", AllowExternalConnectionsCB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
            AccountManager.Instance.RestartWebServer();
        }

        private void PortNumber_ValueChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.WebServer.Set("WebServerPort", PortNumber.Value.ToString());
            AccountManager.IniSettings.Save("RAMSettings.ini");
            AccountManager.Instance.RestartWebServer();
        }

        private void PasswordTextBox_TextChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            PasswordTextBox.Text = Regex.Replace(PasswordTextBox.Text, "[^0-9a-zA-Z ]", "");

            AccountManager.WebServer.Set("Password", PasswordTextBox.Text);
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        #endregion

        #region Miscellaneous

        private void PresenceCB_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.General.Set("ShowPresence", PresenceCB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
            AccountManager.Instance.RefreshAccountsView();
        }

        private void PresenceUpdateRateNum_ValueChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.General.Set("PresenceUpdateRate", PresenceUpdateRateNum.Value.ToString());
            AccountManager.IniSettings.Save("RAMSettings.ini");
            AccountManager.Instance.ApplyPresenceRate();
        }

        private void UnlockFPSCB_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.General.Set("UnlockFPS", UnlockFPSCB.Checked ? "true" : "false");
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        private void MaxFPSValue_ValueChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            AccountManager.General.Set("MaxFPSValue", MaxFPSValue.Value.ToString());
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        private void OverrideWithCustomCB_CheckedChanged(object sender, EventArgs e)
        {
            if (!SettingsLoaded) return;

            UnlockFPSCB.Enabled = !OverrideWithCustomCB.Checked;

            void Remove()
            {
                AccountManager.General.RemoveProperty("CustomClientSettings");
                OverrideWithCustomCB.Checked = false;
            }

            if (OverrideWithCustomCB.Checked)
            {
                if (CustomClientSettingsDialog.ShowDialog() == DialogResult.OK)
                {
                    if (File.Exists(CustomClientSettingsDialog.FileName) && File.ReadAllText(CustomClientSettingsDialog.FileName).TryParseJson<object>(out _))
                    {
                        string FileName = Path.Combine(Environment.CurrentDirectory, "CustomClientAppSettings.json");

                        File.Copy(CustomClientSettingsDialog.FileName, FileName);
                        AccountManager.General.Set("CustomClientSettings", FileName);
                    }
                    else
                        MessageBox.Show("Invalid file selected, make sure it contains valid JSON", "Settings", MessageBoxButtons.OK, MessageBoxIcon.Error);
                }
                else
                    Remove();
            }
            else
                Remove();
            
            AccountManager.IniSettings.Save("RAMSettings.ini");
        }

        #endregion

        #region Themes

        public void ApplyTheme()
        {
            BackColor = ThemeEditor.FormsBackground;
            ForeColor = ThemeEditor.FormsForeground;

            ApplyTheme(Controls);
            RBX_Alt_Manager.Classes.ModernUI.Apply(this);
        }

        public void ApplyTheme(Control.ControlCollection _Controls)
        {
            foreach (Control control in _Controls)
            {
                if (control is Button || control is CheckBox)
                {
                    if (control is Button)
                    {
                        Button b = control as Button;
                        b.FlatStyle = ThemeEditor.ButtonStyle;
                        b.FlatAppearance.BorderColor = ThemeEditor.ButtonsBorder;
                    }

                    if (!(control is CheckBox)) control.BackColor = ThemeEditor.ButtonsBackground;
                    control.ForeColor = ThemeEditor.ButtonsForeground;
                }
                else if (control is TextBox || control is RichTextBox)
                {
                    if (control is Classes.BorderedTextBox)
                    {
                        Classes.BorderedTextBox b = control as Classes.BorderedTextBox;
                        b.BorderColor = ThemeEditor.TextBoxesBorder;
                    }

                    if (control is Classes.BorderedRichTextBox)
                    {
                        Classes.BorderedRichTextBox b = control as Classes.BorderedRichTextBox;
                        b.BorderColor = ThemeEditor.TextBoxesBorder;
                    }

                    control.BackColor = ThemeEditor.TextBoxesBackground;
                    control.ForeColor = ThemeEditor.TextBoxesForeground;
                }
                else if (control is Label)
                {
                    control.BackColor = ThemeEditor.LabelTransparent ? Color.Transparent : ThemeEditor.LabelBackground;
                    control.ForeColor = ThemeEditor.LabelForeground;
                }
                else if (control is ListBox)
                {
                    control.BackColor = ThemeEditor.ButtonsBackground;
                    control.ForeColor = ThemeEditor.ButtonsForeground;
                }
                else if (control is TabPage)
                {
                    ApplyTheme(control.Controls);

                    control.BackColor = ThemeEditor.ButtonsBackground;
                    control.ForeColor = ThemeEditor.ButtonsForeground;
                }
                else if (control is FastColoredTextBoxNS.FastColoredTextBox)
                    control.ForeColor = Color.Black;
                else if (control is FlowLayoutPanel || control is Panel || control is TabControl)
                    ApplyTheme(control.Controls);
            }
        }

        #endregion
    }
}