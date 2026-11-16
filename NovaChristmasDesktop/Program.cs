using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
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

    private readonly System.Windows.Forms.Timer animation = new() { Interval = 30 };
    private readonly Random random = new();
    private readonly List<Flake> snow = new();
    private readonly NotifyIcon tray = new();
    private bool showSnow = true;
    private bool showLights = true;
    private float phase;

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

        for (int i = 0; i < 125; i++) snow.Add(NewFlake(true));

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

    private Flake NewFlake(bool anywhere) => new()
    {
        X = random.Next(Math.Max(1, Width)),
        Y = anywhere ? random.Next(Math.Max(1, Height)) : -10,
        Size = (float)(1.5 + random.NextDouble() * 4.2),
        Speed = (float)(0.8 + random.NextDouble() * 2.5),
        Drift = (float)(-0.55 + random.NextDouble() * 1.1),
        Opacity = (float)(0.45 + random.NextDouble() * 0.5)
    };

    private void TickAnimation()
    {
        phase += 0.055f;
        if (showSnow)
        {
            foreach (var f in snow)
            {
                f.Y += f.Speed;
                f.X += f.Drift + (float)Math.Sin(phase + f.Y / 70f) * 0.35f;
                if (f.Y > Height + 12) { f.Y = -12; f.X = random.Next(Math.Max(1, Width)); }
                if (f.X < -10) f.X = Width + 10;
                if (f.X > Width + 10) f.X = -10;
            }
        }
        Invalidate();
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        base.OnPaint(e);
        var g = e.Graphics;
        g.SmoothingMode = SmoothingMode.AntiAlias;

        if (showLights) DrawGarland(g, true, 42);
        if (showLights) DrawGarland(g, false, 34);

        if (showSnow)
        {
            foreach (var f in snow)
            {
                using var brush = new SolidBrush(Color.FromArgb((int)(255 * f.Opacity), 255, 255, 255));
                g.FillEllipse(brush, f.X, f.Y, f.Size, f.Size);
                if (f.Size > 4.4f)
                {
                    using var glow = new SolidBrush(Color.FromArgb(28, 255, 255, 255));
                    g.FillEllipse(glow, f.X - 3, f.Y - 3, f.Size + 6, f.Size + 6);
                }
            }
        }
    }

    private void DrawGarland(Graphics g, bool top, int depth)
    {
        int width = Width;
        float baseY = top ? 1 : Height - 1;
        using var wire = new Pen(Color.FromArgb(225, 28, 75, 42), 3);
        using var wireHi = new Pen(Color.FromArgb(130, 130, 170, 105), 1);
        using var branch = new Pen(Color.FromArgb(220, 24, 91, 48), 5);
        using var branchHi = new Pen(Color.FromArgb(150, 56, 125, 67), 2);

        var path = new GraphicsPath();
        int segments = Math.Max(12, width / 45);
        PointF PointAt(int i)
        {
            float x = width * i / (float)segments;
            float sag = (float)Math.Sin(i * Math.PI / segments) * depth;
            float wave = (float)Math.Sin(i * Math.PI * 2 / 5.2 + phase * 0.18) * 4;
            return new PointF(x, baseY + (top ? sag + wave : -sag - wave));
        }
        path.AddLines(Enumerable.Range(0, segments + 1).Select(PointAt).ToArray());
        g.DrawPath(wire, path);
        g.DrawPath(wireHi, path);

        for (int x = 8; x < width; x += 24)
        {
            float sag = (float)Math.Sin(x / (float)Math.Max(1, width) * Math.PI) * depth;
            float y = baseY + (top ? sag : -sag);
            for (int side = -1; side <= 1; side++)
            {
                float yy = y + (top ? 1 : -1) * side * 4;
                g.DrawLine(branch, x, yy, x + 10, yy + (top ? 7 : -7));
                g.DrawLine(branchHi, x + 2, yy + (top ? 1 : -1), x + 9, yy + (top ? 6 : -6));
            }
        }

        Color[] colors = { Color.OrangeRed, Color.Gold, Color.DeepSkyBlue, Color.LimeGreen, Color.HotPink, Color.White };
        int index = 0;
        for (int x = 14; x < width; x += 38, index++)
        {
            float sag = (float)Math.Sin(x / (float)Math.Max(1, width) * Math.PI) * depth;
            float y = baseY + (top ? sag : -sag);
            float drop = top ? 13 + index % 3 * 2 : -13 - index % 3 * 2;
            float pulse = 0.72f + 0.28f * (float)Math.Sin(phase * 2.2f + index * 0.8f);
            Color color = colors[index % colors.Length];
            using var glow = new SolidBrush(Color.FromArgb((int)(55 * pulse), color));
            using var bulb = new SolidBrush(Color.FromArgb((int)(235 * pulse), color));
            using var cap = new SolidBrush(Color.FromArgb(220, 65, 65, 65));
            float bx = x, by = y + drop;
            g.FillEllipse(glow, bx - 8, by - 8, 16, 16);
            g.FillRectangle(cap, bx - 1.5f, by - (top ? 5 : -1), 3, 4);
            g.FillEllipse(bulb, bx - 3.2f, by - 3.2f, 6.4f, 6.4f);
        }
        path.Dispose();
    }
}
