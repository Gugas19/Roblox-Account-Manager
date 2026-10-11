using RBX_Alt_Manager.Classes;
using System.Drawing;
using System.Windows.Forms;

namespace RBX_Alt_Manager
{
    /// <summary>
    /// Modern layout of the account utilities window: one section per task, placeholders instead of loose labels
    /// </summary>
    public partial class AccountUtils
    {
        private void ApplyModernLayout()
        {
            SuspendLayout();

            const int Pad = 16, Gap = 8, Width = 580, Field = 23, Button = 30, ActionWidth = 190;
            int Inner = Width - Pad * 2;
            int Y = 12;

            MinimumSize = Size.Empty;
            ClientSize = new Size(Width, 460); // width first, so right-anchored controls are placed against the final edge

            var TL = AnchorStyles.Top | AnchorStyles.Left;
            var TLR = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right;
            var TR = AnchorStyles.Top | AnchorStyles.Right;

            void Place(Control c, int X, int Top, int W, int H, AnchorStyles Anchor) { c.Anchor = Anchor; c.Bounds = new Rectangle(X, Top, W, H); }
            void Section(string Text)
            {
                Controls.Add(new Label { Text = Text, Name = "Section" + Text.Replace(" ", ""), AutoSize = false, Bounds = new Rectangle(Pad, Y, Inner, 16), Anchor = TLR, Font = new Font(ModernUI.FontName, 7.75f, FontStyle.Bold), Tag = "UseControlFont" });
                Y += 20;
            }
            void Row(Control Input, params Control[] Buttons)
            {
                int Right = Width - Pad;

                for (int i = Buttons.Length - 1; i >= 0; i--)
                {
                    int W = Buttons.Length == 1 ? ActionWidth : 155;
                    Place(Buttons[i], Right - W, Y, W, Button, TR);
                    Right -= W + Gap;
                }

                if (Input != null) Place(Input, Pad, Y + (Button - Field) / 2, Right - Pad, Field, TLR);

                Y += Button + 16;
            }

            label1.Visible = label2.Visible = label4.Visible = false;
            textBox3.Text = textBox5.Text = string.Empty;

            Section("PASSWORD");
            int Half = (Inner - ActionWidth - Gap * 2) / 2;
            Place(textBox1, Pad, Y + (Button - Field) / 2, Half, Field, TL);
            Place(textBox2, Pad + Half + Gap, Y + (Button - Field) / 2, Half, Field, TL);
            Place(button1, Width - Pad - ActionWidth, Y, ActionWidth, Button, TR);
            Y += Button + 16;

            Section("EMAIL");
            Row(textBox3, button3);

            Section("DISPLAY NAME");
            Row(DisplayName, SetDisplayName);

            Section("OTHER PLAYERS");
            Row(Username, AddFriend, Block);

            Section("SECURITY");
            Place(textBox5, Pad, Y + (Button - Field) / 2, 90, Field, TL);
            Place(button7, Pad + 90 + Gap, Y, 150, Button, TL);
            Y += Button + 12;

            // follow privacy on its own row: label, then the list right after it
            Label FollowLabel = new Label { Text = "Who Can Follow", Name = "WhoFollowLabel", AutoSize = true, Anchor = TL, Location = new Point(Pad, Y + 7) };
            Controls.Add(FollowLabel);
            WhoFollow.DropDownStyle = ComboBoxStyle.DropDownList;
            Place(WhoFollow, Pad + 130, Y + 3, 240, Field, TL);
            FollowLabel.SizeChanged += (s, e) => WhoFollow.Left = FollowLabel.Right + 10;
            Y += Button + 12;

            Place(button2, Pad, Y, 300, Button, TL);
            Y += Button + 14;

            label3.AutoSize = true;
            label3.Anchor = TL;
            label3.Location = new Point(Pad, Y);
            Y += 36;

            ModernUI.SetPlaceholder(textBox1, "Current Password");
            ModernUI.SetPlaceholder(textBox2, "New Password");
            ModernUI.SetPlaceholder(textBox3, "New email");
            ModernUI.SetPlaceholder(DisplayName, "Display name");
            ModernUI.SetPlaceholder(Username, "Target Username");
            ModernUI.SetPlaceholder(textBox5, "PIN");

            ClientSize = new Size(Width, Y);
            MinimumSize = Size;

            ResumeLayout(true);
        }
    }
}
