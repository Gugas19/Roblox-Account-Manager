using RBX_Alt_Manager.Classes;
using System;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace RBX_Alt_Manager
{
    /// <summary>
    /// Modern layout of the Utilities window: a consistent toolbar row on every tab and placeholders instead of loose labels
    /// </summary>
    public partial class ServerList
    {
        private void ApplyModernLayout()
        {
            SuspendLayout();

            int W = ServersTab.ClientSize.Width, H = ServersTab.ClientSize.Height;
            const int Pad = 8, Row = 30, Field = 23, Top = 8, ContentTop = 46;
            var TL = AnchorStyles.Top | AnchorStyles.Left;
            var TR = AnchorStyles.Top | AnchorStyles.Right;
            var Fill = AnchorStyles.Top | AnchorStyles.Bottom | AnchorStyles.Left | AnchorStyles.Right;

            void Place(Control c, int X, int Y, int Width, int Height, AnchorStyles Anchor) { c.Anchor = Anchor; c.Bounds = new Rectangle(X, Y, Width, Height); }
            int FieldY = Top + (Row - Field) / 2;

            // Servers
            Place(RefreshServers, Pad, Top, 112, Row, TL);
            Place(OtherPlaceId, Pad + 112 + Pad, FieldY, 200, Field, TL);
            UsernameLabel.Visible = false;
            Place(SearchPlayer, W - Pad - 120, Top, 120, Row, TR);
            Place(Username, W - Pad - 120 - Pad - 200, FieldY, 200, Field, TR);
            Place(ServerListView, Pad, ContentTop, W - Pad * 2, H - ContentTop - Pad, Fill);
            ModernUI.SetPlaceholder(OtherPlaceId, "Other Place ID (optional)");
            ModernUI.SetPlaceholder(Username, "Find player by username");

            // Games
            Place(Search, W - Pad - 110, Top, 110, Row, TR);
            Place(PageNum, W - Pad - 110 - Pad - 48, FieldY, 48, Field, TR);
            label1.AutoSize = true;
            label1.Anchor = TR;
            label1.Location = new Point(PageNum.Left - 44, FieldY + 4);
            ListViewCB.Anchor = TR;
            ListViewCB.Location = new Point(label1.Left - 100, FieldY + 3);
            Place(Term, Pad, FieldY, ListViewCB.Left - Pad * 3, Field, AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right);
            Place(GameListPanel, Pad, ContentTop, W - Pad * 2, H - ContentTop - Pad, Fill);
            Place(GamesListView, Pad, ContentTop, W - Pad * 2, H - ContentTop - Pad, Fill);
            ModernUI.SetPlaceholder(Term, "Search games...");

            // Favorites
            Place(Favorite, Pad, Top, 200, Row, TL);
            FavoriteListViewCB.Location = new Point(Pad + 200 + Pad * 2, FieldY + 3);
            Place(FavoriteGamesPanel, Pad, ContentTop, W - Pad * 2, H - ContentTop - Pad, Fill);
            Place(FavoritesListView, Pad, ContentTop, W - Pad * 2, H - ContentTop - Pad, Fill);

            // Universe
            uPlaceIDLabel.Visible = false;
            uUniverseIdLabel.Visible = false;
            Place(PlaceIDUniTB, Pad, FieldY, 120, Field, TL);
            Place(GetUniverseID, Pad + 120 + Pad, Top, 150, Row, TL);
            Place(ViewUniverse, W - Pad - 130, Top, 130, Row, TR);
            Place(UniverseIDTB, W - Pad - 130 - Pad - 120, FieldY, 120, Field, TR);
            Place(UniverseGamesPanel, Pad, ContentTop, W - Pad * 2, H - ContentTop - Pad, Fill);
            ModernUI.SetPlaceholder(PlaceIDUniTB, "Place ID");
            ModernUI.SetPlaceholder(UniverseIDTB, "Universe ID");

            // Outfits
            OutfitUsernameLabel.Visible = false;
            Place(OutfitUsernameTB, Pad, FieldY, 170, Field, TL);
            Place(ViewOutfits, Pad + 170 + Pad, Top, 130, Row, TL);
            Place(WearCustomButton, W - Pad - 140, Top, 140, Row, TR);
            Place(OutfitsPanel, Pad, ContentTop, W - Pad * 2, H - ContentTop - Pad, Fill);
            ModernUI.SetPlaceholder(OutfitUsernameTB, "Username");

            // Watcher: evenly spaced rows, inline controls are positioned by FixInlineLayout once translated
            Control[][] Rows =
            {
                new Control[] { RobloxScannerCB },
                new Control[] { ScanESLabel, ScanIntervalN },
                new Control[] { label2, ReadIntervalN },
                new Control[] { ExitIfBetaDetectedCB },
                new Control[] { ExitIfNoConnectionCB, TimeoutNum, ConnectionSecondsLabel },
                new Control[] { SaveWindowPositionsCB },
                new Control[] { VerifyDataModelCB },
                new Control[] { IgnoreExistingProcesses },
                new Control[] { RbxMemoryCB, RbxMemoryLTNum, MBLabel },
                new Control[] { CloseRbxWindowTitleCB, RbxWindowNameTB },
            };

            for (int i = 0; i < Rows.Length; i++)
                foreach (Control control in Rows[i])
                    control.Top = 14 + i * 28 + (control is Label ? 4 : control is CheckBox ? 2 : 0);

            OpenLogsButton.Text = string.Empty;

            ClientSize = new Size(ClientSize.Width + 180, ClientSize.Height + 120);
            MinimumSize = new Size(MinimumSize.Width + 120, MinimumSize.Height + 80);
            Place(OpenLogsButton, ClientSize.Width - 44, ClientSize.Height - 44, 34, 34, AnchorStyles.Bottom | AnchorStyles.Right);
            SaveTooltip(OpenLogsButton, "Open Roblox logs folder");

            ResumeLayout(true);
        }

        private ToolTip LogsTip;

        private void SaveTooltip(Control control, string Text)
        {
            LogsTip ??= new ToolTip(components);
            LogsTip.SetToolTip(control, Text);
        }

        /// <summary>Keeps controls that sit on the same row from overlapping after texts were translated</summary>
        private void FixInlineLayout()
        {
            int Gap = (int)(8 * Program.Scale);

            int Right(Control control) => control.Left + control.PreferredSize.Width; // measured from the text, the size may not be updated yet
            void After(Control Previous, Control Next) => Next.Left = Right(Previous) + Gap;

            int LabelsRight = Math.Max(Right(ScanESLabel), Right(label2));
            ScanIntervalN.Left = ReadIntervalN.Left = LabelsRight + Gap;

            After(ExitIfNoConnectionCB, TimeoutNum);
            After(TimeoutNum, ConnectionSecondsLabel);
            After(RbxMemoryCB, RbxMemoryLTNum);
            After(RbxMemoryLTNum, MBLabel);
            After(CloseRbxWindowTitleCB, RbxWindowNameTB);
            RbxWindowNameTB.Width = (int)(110 * Program.Scale);
        }
    }
}
