"""把每套预设真正部署一遍，抓小狼毫画出来的候选框，存成缩略图。

界面上那些预设图不是前端仿的，是这个脚本跑出来的真实截图：逐套写进
weasel.custom.yaml → WeaselDeployer /deploy → 敲几个字母 → 抓候选框窗口。

跑之前先把用户原来的 weasel.custom.yaml 备份，跑完原样还回去并重新部署。

只能在装了小狼毫的 Windows 上跑，另外需要 comtypes（切输入法用）：

    pip install comtypes
    python scripts/shoot-presets.py

改了 src/data/presets.json 之后重跑一次。
"""
import ctypes
import json
import io
import os
import shutil
import subprocess
import sys
import time
import tkinter as tk
from ctypes import wintypes

from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, 'src', 'assets', 'presets')
RIME = os.path.join(os.environ['APPDATA'], 'Rime')
CUSTOM = os.path.join(RIME, 'weasel.custom.yaml')

u = ctypes.WinDLL('user32', use_last_error=True)
gdi = ctypes.WinDLL('gdi32', use_last_error=True)
ENUM = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
KEYEVENTF_KEYUP = 0x0002
SPI_GETFOREGROUNDLOCKTIMEOUT, SPI_SETFOREGROUNDLOCKTIMEOUT = 0x2000, 0x2001
SRCCOPY = 0x00CC0020

HOST_BG = '#ff00ff'   # 裁边用的哨兵色，预设里不可能出现
KEYS = 'YUSE'          # 打出来是「预设 语塞 淤塞……」，候选够多，看得出间距


# ── 配置 ────────────────────────────────────────────────────────────────
# 键名和写法必须跟 src-tauri/src/config.rs 的 build_yaml_patch 保持一致。
# 尤其颜色一定是 `0xAABBGGRR` 十六进制串：写十进制的话小狼毫走
# config_get_int，带 alpha 的值超过 INT_MAX 解析失败，整项退回默认色。
COLOR_KEYS = {
    'backColor': 'back_color', 'borderColor': 'border_color',
    'textColor': 'text_color', 'hilitedTextColor': 'hilited_text_color',
    'hilitedBackColor': 'hilited_back_color',
    'candidateTextColor': 'candidate_text_color',
    'commentTextColor': 'comment_text_color', 'labelColor': 'label_color',
    'hilitedCandidateTextColor': 'hilited_candidate_text_color',
    'hilitedCandidateBackColor': 'hilited_candidate_back_color',
    'hilitedCommentTextColor': 'hilited_comment_text_color',
}
LAYOUT_KEYS = {
    'borderWidth': 'border_width', 'candidateSpacing': 'candidate_spacing',
    'cornerRadius': 'corner_radius', 'hilitePaddingX': 'hilite_padding_x',
    'hilitePaddingY': 'hilite_padding_y', 'hiliteSpacing': 'hilite_spacing',
    'marginX': 'margin_x', 'marginY': 'margin_y', 'maxWidth': 'max_width',
    'minWidth': 'min_width', 'roundCorner': 'round_corner',
    'shadowRadius': 'shadow_radius', 'spacing': 'spacing',
}


def bgr(hexcolor):
    c = hexcolor.lstrip('#')
    r, g, b = int(c[0:2], 16), int(c[2:4], 16), int(c[4:6], 16)
    return '0x%08X' % (0xFF000000 | (b << 16) | (g << 8) | r)


def write_config(preset):
    t, l = preset['theme'], preset['layout']
    lines = ['patch:', '  preset_color_schemes/xgrime_shot:',
             '    name: XGRime 截图用']
    for src, dst in COLOR_KEYS.items():
        lines.append(f'    {dst}: {bgr(t[src])}')
    label = bgr(t['hilitedCandidateLabelColor'])
    lines.append(f'    hilited_label_color: {label}')
    lines.append(f'    hilited_candidate_label_color: {label}')
    if t.get('hilitedMarkColor'):
        lines.append(f"    hilited_mark_color: {bgr(t['hilitedMarkColor'])}")
    lines += ['  style/color_scheme: xgrime_shot',
              '  style/color_scheme_dark: xgrime_shot',
              '  show_notifications: false',
              f"  style/horizontal: {'true' if l['horizontal'] else 'false'}",
              f"  style/inline_preedit: {'true' if l['inlinePreedit'] else 'false'}",
              f"  style/font_point: {l['fontSize']}",
              f"  style/label_font_point: {l['labelFontSize'] or l['fontSize']}",
              f"  style/label_format: '{l['labelFormat']}'",
              f"  style/mark_text: '{l['markText']}'"]
    for src, dst in LAYOUT_KEYS.items():
        lines.append(f'  style/layout/{dst}: {l[src]}')
    io.open(CUSTOM, 'w', encoding='utf-8', newline='\n').write('\n'.join(lines) + '\n')


def deploy(exe):
    subprocess.run([exe, '/deploy'], capture_output=True)


def deployer():
    import winreg
    for view in (winreg.KEY_WOW64_32KEY, winreg.KEY_WOW64_64KEY):
        try:
            k = winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, r'SOFTWARE\Rime\Weasel',
                               0, winreg.KEY_READ | view)
            root = winreg.QueryValueEx(k, 'WeaselRoot')[0]
            exe = os.path.join(root, 'WeaselDeployer.exe')
            if os.path.exists(exe):
                return exe
        except OSError:
            continue
    return None


# ── 窗口 ────────────────────────────────────────────────────────────────
class KEYBDINPUT(ctypes.Structure):
    _fields_ = [('wVk', wintypes.WORD), ('wScan', wintypes.WORD),
                ('dwFlags', wintypes.DWORD), ('time', wintypes.DWORD),
                ('dwExtraInfo', ctypes.POINTER(ctypes.c_ulong))]


class INPUT(ctypes.Structure):
    class _U(ctypes.Union):
        _fields_ = [('ki', KEYBDINPUT), ('pad', ctypes.c_byte * 32)]
    _anonymous_ = ('u',)
    _fields_ = [('type', wintypes.DWORD), ('u', _U)]


def pid_of(h):
    p = wintypes.DWORD()
    u.GetWindowThreadProcessId(h, ctypes.byref(p))
    return p.value


def pump(root, seconds):
    end = time.time() + seconds
    while time.time() < end:
        root.update()
        time.sleep(0.02)


def key(vk, guard_pid):
    # 前台不是我们自己的窗口就停手，免得按键打到别的程序上
    if pid_of(u.GetForegroundWindow()) != guard_pid:
        raise RuntimeError('前台不是抓图窗口，停手')
    arr = (INPUT * 2)()
    for i, f in enumerate((0, KEYEVENTF_KEYUP)):
        arr[i].type = 1
        arr[i].ki = KEYBDINPUT(vk, 0, f, 0, None)
    u.SendInput(2, ctypes.byref(arr), ctypes.sizeof(INPUT))


def force_foreground(hwnd):
    old = wintypes.UINT()
    u.SystemParametersInfoW(SPI_GETFOREGROUNDLOCKTIMEOUT, 0, ctypes.byref(old), 0)
    u.SystemParametersInfoW(SPI_SETFOREGROUNDLOCKTIMEOUT, 0, ctypes.c_void_p(0), 0)
    try:
        fg = u.GetForegroundWindow()
        other = u.GetWindowThreadProcessId(fg, None)
        mine = ctypes.WinDLL('kernel32').GetCurrentThreadId()
        u.AttachThreadInput(mine, other, True)
        u.BringWindowToTop(hwnd)
        u.SetForegroundWindow(hwnd)
        u.SetFocus(hwnd)
        u.AttachThreadInput(mine, other, False)
    finally:
        u.SystemParametersInfoW(SPI_SETFOREGROUNDLOCKTIMEOUT, 0,
                                ctypes.c_void_p(old.value), 0)


def ensure_front(root, top, me):
    for _ in range(8):
        if pid_of(u.GetForegroundWindow()) == me:
            return True
        force_foreground(top)
        pump(root, 0.35)
    return pid_of(u.GetForegroundWindow()) == me


def panel_rect():
    """候选框那个窗口的屏幕矩形；没在显示就返回 None

    别按 WeaselServer 的 pid 去找：TSF 模式下候选框是建在输入端进程里的，
    按 pid 过滤会一个都找不到。认窗口类名（ATL: 开头）加上「宽出来了」就够。
    """
    hit = []

    def cb(h, _):
        if u.IsWindowVisible(h):
            cls = ctypes.create_unicode_buffer(256)
            u.GetClassNameW(h, cls, 256)
            r = wintypes.RECT()
            u.GetWindowRect(h, ctypes.byref(r))
            w, ht = r.right - r.left, r.bottom - r.top
            # 32x32 是候选框没内容时的样子；宽出来才说明真的在显示候选
            # 竖排布局的候选框能有三四百高，只卡下限，别卡上限
            if cls.value.startswith('ATL:') and w > 60 and ht > 20:
                hit.append((r.left, r.top, r.right, r.bottom))
        return True

    u.EnumWindows(ENUM(cb), 0)
    return hit[0] if hit else None


def paint(root, lbl, spacer, entry, color):
    root.configure(bg=color)
    lbl.configure(bg=color, fg=color)
    spacer.configure(bg=color)
    entry.configure(bg=color, fg=color, insertbackground=color,
                    highlightthickness=0, bd=0)


def box_bounds(img):
    """算出候选框在这张图里的位置（去掉窗口那圈阴影余量）

    判据是「这一行/列有多少不是哨兵色」：框是不透明的，框里的行整行都不是
    洋红；外面那圈阴影是半透明的，绝大部分还是洋红。

    别改回按候选框背景色判断 —— 高亮块占满整行时（竖排预设就是），
    那一行一个背景色像素都没有，会被当成边框裁掉，整整少一个候选。
    """
    px = img.convert('RGB').load()
    w, h = img.size

    def sentinel(x, y):
        r, g, b = px[x, y]
        return r > 140 and g < 110 and b > 140

    def solid(fixed, vertical, lo, hi, ratio):
        """这一行（列）落在 [lo, hi) 里的采样点，有多少不是哨兵色"""
        hits = total = 0
        for i in range(lo, hi, 2):
            total += 1
            if not (sentinel(fixed, i) if vertical else sentinel(i, fixed)):
                hits += 1
        return total and hits > total * ratio

    # 分两遍：先拿整幅宽度定出行的范围，再只在这些行里定列。
    # 一遍做完的话，列的分母是整幅高度（框只占其中一段），怎么算都不达标。
    rows = [y for y in range(h) if solid(y, False, 0, w, 0.5)]
    if not rows:
        return None
    cols = [x for x in range(w) if solid(x, True, rows[0], rows[-1] + 1, 0.8)]
    if not cols:
        return None
    rows = [y for y in range(h) if solid(y, False, cols[0], cols[-1] + 1, 0.8)]
    if not rows:
        return None
    return (cols[0], rows[0], cols[-1] + 1, rows[-1] + 1)


def grab(rect, host):
    """只抓屏幕上这一小块，而且必须整块落在我们自己那个窗口里

    候选框是分层窗口，PrintWindow 抓出来全黑，只能从屏幕取。所以先用一个铺满的
    自有窗口把那片区域盖住，再校验候选框确实在窗口范围内 —— 这样每个像素要么是
    候选框，要么是我们自己的窗口，物理上拍不到别人的内容。
    """
    l, t, r, b = rect
    if not (l >= host[0] and t >= host[1] and r <= host[2] and b <= host[3]):
        return None
    w, h = r - l, b - t
    screen = u.GetDC(0)
    mem = gdi.CreateCompatibleDC(screen)
    bmp = gdi.CreateCompatibleBitmap(screen, w, h)
    old = gdi.SelectObject(mem, bmp)
    gdi.BitBlt(mem, 0, 0, w, h, screen, l, t, SRCCOPY)

    class BIH(ctypes.Structure):
        _fields_ = [('biSize', wintypes.DWORD), ('biWidth', wintypes.LONG),
                    ('biHeight', wintypes.LONG), ('biPlanes', wintypes.WORD),
                    ('biBitCount', wintypes.WORD), ('biCompression', wintypes.DWORD),
                    ('biSizeImage', wintypes.DWORD), ('biXPelsPerMeter', wintypes.LONG),
                    ('biYPelsPerMeter', wintypes.LONG), ('biClrUsed', wintypes.DWORD),
                    ('biClrImportant', wintypes.DWORD)]

    class BI(ctypes.Structure):
        _fields_ = [('h', BIH), ('c', wintypes.DWORD * 3)]

    info = BI()
    info.h.biSize = ctypes.sizeof(BIH)
    info.h.biWidth, info.h.biHeight = w, -h
    info.h.biPlanes, info.h.biBitCount = 1, 32
    buf = ctypes.create_string_buffer(w * h * 4)
    gdi.GetDIBits(mem, bmp, 0, h, buf, ctypes.byref(info), 0)
    gdi.SelectObject(mem, old)
    gdi.DeleteObject(bmp)
    gdi.DeleteDC(mem)
    u.ReleaseDC(0, screen)

    return Image.frombuffer('RGB', (w, h), buf, 'raw', 'BGRX', 0, 1)


def main():
    exe = deployer()
    if not exe:
        print('找不到小狼毫的 WeaselDeployer.exe'); return 2
    sys.path.insert(0, os.path.join(ROOT, 'scripts'))
    try:
        import tsf
    except ImportError:
        print('缺 tsf.py（切输入法用），跟这个脚本放同一个目录'); return 2

    presets = json.load(io.open(os.path.join(ROOT, 'src', 'data', 'presets.json'),
                                encoding='utf-8'))
    only = set(sys.argv[1:])
    shoot_list = [x for x in presets if not only or x['key'] in only]
    backup = CUSTOM + '.xgrime-shot-backup'
    if os.path.exists(CUSTOM):
        shutil.copy2(CUSTOM, backup)
        print('原配置已备份到', backup)

    p, profs = tsf.profiles()
    weasel = next((x for x in profs if '小狼毫' in x[0] or 'Weasel' in x[0]), None)
    before = next((x for x in profs if x[4]), None)
    if not weasel:
        print('系统里没有小狼毫这个输入法'); return 2

    me = os.getpid()
    root = tk.Tk()
    root.title('XGRime 抓预设图')
    root.geometry('1100x520+60+60')
    root.configure(bg=HOST_BG)
    lbl = tk.Label(root, text='正在逐套预设抓候选框截图，完事自动关闭',
                   bg=HOST_BG, fg='#ffffff')
    lbl.pack(pady=(16, 8))
    e = tk.Entry(root, font=('Microsoft YaHei', 14))
    e.pack(fill='x', padx=20, ipady=4)
    spacer = tk.Frame(root, bg=HOST_BG)
    spacer.pack(fill='both', expand=True)
    e.focus_force()
    pump(root, 0.5)

    top = u.GetAncestor(int(root.winfo_id()), 2)
    for _ in range(6):
        force_foreground(top)
        pump(root, 0.3)
        if pid_of(u.GetForegroundWindow()) == me:
            break
    if pid_of(u.GetForegroundWindow()) != me:
        print('抢不到前台焦点（多半有全屏程序压着），先把它切走再跑')
        root.destroy(); return 4

    os.makedirs(OUT, exist_ok=True)
    done = {}
    try:
        tsf.activate(p, weasel[1], weasel[2], weasel[3])
        pump(root, 1.0)
        e.focus_force()
        pump(root, 0.5)

        for preset in shoot_list:
            write_config(preset)
            deploy(exe)
            pump(root, 0.8)
            # 部署会弹托盘提示、也可能被别的窗口抢走焦点，每一轮都重新确认一次
            if not ensure_front(root, top, me):
                print(f"  {preset['key']:16} 抢不回焦点，跳过")
                continue
            # 焦点离开再回来，窗口的输入法会退回系统默认（多半是微软拼音），
            # 所以每一轮都重切一次，并且当场确认真的切过去了
            tsf.activate(p, weasel[1], weasel[2], weasel[3])
            pump(root, 0.5)
            e.focus_force()
            pump(root, 0.4)
            now = next((x[0] for x in tsf.profiles()[1] if x[4]), '?')
            if now != weasel[0]:
                print(f"  {preset['key']:16} 输入法没切过去（当前 {now}），跳过")
                continue
            e.delete(0, 'end')
            # 候选框的阴影边是透明的，底下窗口的东西（尤其是正在组字的拼音）
            # 会透出来。窗口一律刷成洋红：任何预设都不会拿它当底色，
            # 裁边时就能干净地把这一圈连同透出来的东西一起切掉。
            paint(root, lbl, spacer, e, HOST_BG)
            pump(root, 0.3)

            rect = None
            for ch in KEYS:
                key(ord(ch), me)
                pump(root, 0.45)
                rect = panel_rect() or rect
            for _ in range(14):
                pump(root, 0.25)
                now = panel_rect()
                if now and (rect is None or now[2] - now[0] >= rect[2] - rect[0]):
                    rect = now

            hr = wintypes.RECT()
            u.GetWindowRect(top, ctypes.byref(hr))
            host = (hr.left, hr.top, hr.right, hr.bottom)
            back = preset['theme']['backColor']
            size = None
            if rect:
                # 第一张洋红底：底色和任何预设都不撞，边界找得准
                bounds = box_bounds(grab(rect, host))
                # 第二张把窗口刷成候选框同色再拍：裁边多留的那一圈就看不出来了
                paint(root, lbl, spacer, e, back)
                pump(root, 0.35)
                shot = grab(rect, host)
                if bounds and shot:
                    shot = shot.crop(bounds)
                if shot:
                    shot.save(os.path.join(OUT, preset['key'] + '.png'))
                    size = shot.size
                paint(root, lbl, spacer, e, HOST_BG)
            print(f"  {preset['key']:16} {size if size else '没抓到（输入框里是 ' + repr(e.get()) + '）'}")
            if size:
                done[preset['key'] + '.png'] = size
            key(0x1B, me)
            pump(root, 0.4)
    finally:
        if before:
            tsf.activate(p, before[1], before[2], before[3])
        root.destroy()
        if os.path.exists(backup):
            shutil.move(backup, CUSTOM)
            deploy(exe)
            print('原配置已还原并重新部署')

    # 只删「不属于任何预设」的图。这轮没抓到的那几张要留着旧图，不然一次失手就没了
    keys = {x['key'] + '.png' for x in presets}
    for stale in set(os.listdir(OUT)) - keys:
        os.remove(os.path.join(OUT, stale))
    print(len(done), '/', len(shoot_list), '张 ->', os.path.relpath(OUT, ROOT))
    return 0 if len(done) == len(shoot_list) else 1


if __name__ == '__main__':
    sys.exit(main())
