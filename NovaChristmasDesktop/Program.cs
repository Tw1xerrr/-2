using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Linq;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace NovaChristmasDesktop;

internal static class Program
{
    [STAThread]
    private static void Main()
    {
        ApplicationConfiguration.Initialize();
        Application.Run(new ChristmasOverlay());
    }
}

internal sealed class ChristmasOverlay : Form
{
    private const int WS_EX_TRANSPARENT = 0x20;
    private const int WS_EX_TOOLWINDOW = 0x80;
    private const int WS_EX_LAYERED = 0x80000;
    private const uint SWP_NOSIZE = 0x0001;
    private const uint SWP_NOMOVE = 0x0002;
    private const uint SWP_NOACTIVATE = 0x0010;
    private const uint SWP_NOOWNERZORDER = 0x0200;

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr FindWindow(string? className, string? windowName);

    [DllImport("user32.dll")]
    private static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter,
        int x, int y, int cx, int cy, uint flags);

    private readonly System.Windows.Forms.Timer animation = new() { Interval = 30 };
    private readonly Random random = new();
    private readonly List<Flake> snow = new();
    private readonly NotifyIcon tray = new();
    private bool showSnow = true;
    private bool showLights = true;
    private float phase;

    private const float CursorRepelRadius = 195f;

    private sealed class Flake
    {
        public float X, Y, Size, Speed, Drift, Opacity;
    }

    public ChristmasOverlay()
    {
        FormBorderStyle = FormBorderStyle.None;
        ShowInTaskbar = false;
        TopMost = true;
        StartPosition = FormStartPosition.Manual;
        Bounds = SystemInformation.VirtualScreen;
        BackColor = Color.Magenta;
        TransparencyKey = Color.Magenta;
        DoubleBuffered = true;

        for (int i = 0; i < 165; i++) snow.Add(NewFlake(true));

        var menu = new ContextMenuStrip();
        var snowToggle = new ToolStripMenuItem("Снег: включён") { CheckOnClick = true, Checked = true };
        var lightsToggle = new ToolStripMenuItem("Гирлянды: включены") { CheckOnClick = true, Checked = true };
        snowToggle.CheckedChanged += (_, _) =>
        {
            showSnow = snowToggle.Checked;
            snowToggle.Text = showSnow ? "Снег: включён" : "Снег: выключен";
            Invalidate();
        };
        lightsToggle.CheckedChanged += (_, _) =>
        {
            showLights = lightsToggle.Checked;
            lightsToggle.Text = showLights ? "Гирлянды: включены" : "Гирлянды: выключены";
            Invalidate();
        };
        menu.Items.Add(snowToggle);
        menu.Items.Add(lightsToggle);
        menu.Items.Add(new ToolStripSeparator());
        menu.Items.Add("Выход", null, (_, _) => Close());

        tray.Icon = SystemIcons.Application;
        tray.Text = "Nova Christmas Desktop";
        tray.ContextMenuStrip = menu;
        tray.Visible = true;

        Shown += (_, _) => KeepBehindTaskbar();
        animation.Tick += (_, _) => TickAnimation();
        animation.Start();
        FormClosed += (_, _) =>
        {
            animation.Stop();
            tray.Visible = false;
            tray.Dispose();
        };
    }

    protected override CreateParams CreateParams
    {
        get
        {
            var cp = base.CreateParams;
            cp.ExStyle |= WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW;
            return cp;
        }
    }

    // Keep the overlay above ordinary windows but immediately behind the Windows taskbar.
    private void KeepBehindTaskbar()
    {
        IntPtr taskbar = FindWindow("Shell_TrayWnd", null);
        if (taskbar != IntPtr.Zero)
            SetWindowPos(Handle, taskbar, 0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER);
    }

    private Flake NewFlake(bool anywhere) => new()
    {
        X = random.Next(Math.Max(1, Width)),
        Y = anywhere ? random.Next(Math.Max(1, Height)) : -8,
        Size = (float)(0.9 + random.NextDouble() * 2.0),
        Speed = (float)(0.65 + random.NextDouble() * 1.65),
        Drift = (float)(-0.38 + random.NextDouble() * 0.76),
        Opacity = (float)(0.78 + random.NextDouble() * 0.22)
    };

    private void TickAnimation()
    {
        phase += 0.055f;

        if (showSnow)
        {
            Point cursor = Cursor.Position;
            float cursorX = cursor.X - Bounds.Left;
            float cursorY = cursor.Y - Bounds.Top;

            foreach (var f in snow)
            {
                f.Y += f.Speed;
                f.X += f.Drift + (float)Math.Sin(phase + f.Y / 70f) * 0.22f;

                // Soft invisible force field around the mouse pointer.
                float dx = f.X - cursorX;
                float dy = f.Y - cursorY;
                float distanceSquared = dx * dx + dy * dy;
                float radiusSquared = CursorRepelRadius * CursorRepelRadius;

                if (distanceSquared < radiusSquared && distanceSquared > 0.01f)
                {
                    float distance = (float)Math.Sqrt(distanceSquared);
                    float falloff = 1f - distance / CursorRepelRadius;
                    float push = 4.6f * falloff * falloff;
                    f.X += dx / distance * push;
                    f.Y += dy / distance * push;
                }

                if (f.Y > Height + 8) { f.Y = -8; f.X = random.Next(Math.Max(1, Width)); }
                if (f.X < -8) f.X = Width + 8;
                if (f.X > Width + 8) f.X = -8;
            }
        }

        Invalidate();
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        base.OnPaint(e);
        var g = e.Graphics;
        // Use hard-edged drawing for the overlay: its magenta color-key transparency
        // can otherwise leave pink fringes around anti-aliased greenery and bulbs.
        g.SmoothingMode = SmoothingMode.None;
        g.PixelOffsetMode = PixelOffsetMode.None;

        if (showLights)
        {
            DrawGarland(g, top: true);
            DrawGarland(g, top: false);
        }

        if (showSnow)
        {
            // Draw snow as solid pure-white pixels without magenta anti-aliased fringes.
            // The magenta transparency key can otherwise tint the partially covered edge pixels.
            var oldSmoothing = g.SmoothingMode;
            var oldPixelOffset = g.PixelOffsetMode;
            g.SmoothingMode = SmoothingMode.None;
            g.PixelOffsetMode = PixelOffsetMode.None;
            using var snowBrush = new SolidBrush(Color.White);
            foreach (var f in snow)
            {
                float size = Math.Max(1.5f, f.Size);
                g.FillEllipse(snowBrush, (float)Math.Round(f.X), (float)Math.Round(f.Y), size, size);
            }
            g.SmoothingMode = oldSmoothing;
            g.PixelOffsetMode = oldPixelOffset;
        }
    }

    private void DrawGarland(Graphics g, bool top)
    {
        int width = Width;
        // The lower strand is tucked into the top edge of the taskbar. The shell taskbar
        // sits above this window, hiding the cable and the lower half of the greenery.
        float baseY = top ? 0f : Height - 44f;
        float depth = top ? 19f : 15f;
        int segments = Math.Max(16, width / 34);

        PointF PointAt(int i)
        {
            float x = width * i / (float)segments;
            float sag = (float)Math.Sin(i * Math.PI / segments) * depth;
            return new PointF(x, baseY + (top ? sag : -sag));
        }

        using var cable = new Pen(Color.FromArgb(205, 42, 54, 35), top ? 1.3f : 1.7f);
        using var cableHighlight = new Pen(Color.FromArgb(105, 154, 145, 104), 0.65f);
        using var needlesDark = new Pen(Color.FromArgb(225, 19, 72, 39), top ? 2.2f : 3.1f);
        using var needlesMid = new Pen(Color.FromArgb(220, 34, 105, 53), top ? 1.8f : 2.5f);
        using var needlesLight = new Pen(Color.FromArgb(155, 73, 133, 69), 1.0f);
        using var branchSnow = new Pen(Color.FromArgb(255, 255, 255, 255), 1.25f);
        using var snowDot = new SolidBrush(Color.White);

        using var path = new GraphicsPath();
        path.AddLines(Enumerable.Range(0, segments + 1).Select(PointAt).ToArray());
        g.DrawPath(cable, path);
        g.DrawPath(cableHighlight, path);

        // Short overlapping fir sprigs, angled down from the wire for a more natural silhouette.
        int spacing = top ? 13 : 10;
        for (int x = 3; x < width; x += spacing)
        {
            float t = x / (float)Math.Max(1, width);
            float sag = (float)Math.Sin(t * Math.PI) * depth;
            float y = baseY + (top ? sag : -sag);
            float sway = (float)Math.Sin(x * 0.13f) * 1.7f;
            float direction = top ? 1f : -1f;

            for (int side = -1; side <= 1; side++)
            {
                float sx = x + side * 2.5f;
                float sy = y + sway + side * 1.5f;
                float ex = sx + (side == 0 ? 8 : 6) * (side < 0 ? -1 : 1);
                float ey = sy + direction * (top ? 9 : 11);

                g.DrawLine(needlesDark, sx, sy, ex, ey);
                g.DrawLine(needlesMid, sx + 1, sy + direction * 1.5f, ex - 1, ey - direction);
                g.DrawLine(needlesLight, sx + 1, sy + direction * 2.5f, ex - 2, ey - direction * 2);

                // Crisp white snow resting on the upper edge of each fir sprig.
                float snowY = Math.Min(sy, ey) + 2.5f;
                g.DrawLine(branchSnow, sx + 1, snowY, ex - 2, snowY + direction * 1.5f);
                if ((x / spacing + side + 4) % 3 == 0)
                    g.FillEllipse(snowDot, ex - 1.5f, ey - 2.5f, 2.2f, 2.2f);
            }
        }

        // Small, warm bulbs spaced farther apart on the slimmer upper strand.
        Color[] colors = { Color.FromArgb(255, 224, 112, 82), Color.FromArgb(255, 245, 207, 111),
            Color.FromArgb(255, 131, 190, 218), Color.FromArgb(255, 159, 202, 132) };
        int bulbSpacing = top ? 56 : 46;
        int index = 0;
        for (int x = top ? 42 : 28; x < width; x += bulbSpacing, index++)
        {
            float t = x / (float)Math.Max(1, width);
            float sag = (float)Math.Sin(t * Math.PI) * depth;
            float y = baseY + (top ? sag : -sag);
            float drop = top ? 7f : -7f;
            float pulse = 0.82f + 0.18f * (float)Math.Sin(phase * 1.8f + index * 1.13f);
            Color color = colors[index % colors.Length];
            float radius = top ? 2.5f : 3.0f;

            using var glow = new SolidBrush(Color.FromArgb((int)(34 * pulse), color));
            using var bulb = new SolidBrush(Color.FromArgb((int)(225 * pulse), color));
            using var socket = new SolidBrush(Color.FromArgb(210, 58, 62, 47));
            float bx = x;
            float by = y + drop;
            g.FillEllipse(glow, bx - radius * 2.5f, by - radius * 2.5f, radius * 5f, radius * 5f);
            g.FillRectangle(socket, bx - 1f, by - (top ? 4f : -1f), 2f, 3f);
            g.FillEllipse(bulb, bx - radius, by - radius, radius * 2f, radius * 2f);
        }
    }
}
