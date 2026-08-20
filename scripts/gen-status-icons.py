"""生成内置的状态图标：中文态「中」、英文态「A」、全角「全」、半角「半」。

小狼毫要的是磁盘上的图标文件，不是网页图标字体，所以这里离线渲染好、
连同应用一起打包：

    src-tauri/resources/status-icons/<套>/{zhung,ascii,full,half}.ico   给小狼毫
    src/assets/status-icons/<套>-<状态>.png                             给界面图库

每个尺寸单独渲染再拼成 .ico，不从大图缩 —— 托盘那格就 16px，缩出来的字糊成一团。

    python scripts/gen-status-icons.py
"""
import io
import os
import struct
import sys

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ICO_OUT = os.path.join(ROOT, 'src-tauri', 'resources', 'status-icons')
PNG_OUT = os.path.join(ROOT, 'src', 'assets', 'status-icons')

CJK = r'C:\Windows\Fonts\msyhbd.ttc'          # 微软雅黑 Bold
LATIN = r'C:\Windows\Fonts\segoeuib.ttf'      # Segoe UI Bold

# 小狼毫认的四个状态键
STATES = [
    ('zhung', '中'),
    ('ascii', 'A'),
    ('full', '全'),
    ('half', '半'),
]

SIZES = [16, 20, 24, 32, 48]

# 底色为 None 就是透明底，只有字
SETS = {
    'plain_dark': {'fg': (26, 26, 26, 255), 'bg': None},
    'plain_light': {'fg': (255, 255, 255, 255), 'bg': None},
    'badge_blue': {'fg': (255, 255, 255, 255), 'bg': (0, 103, 192, 255)},
    'badge_ink': {'fg': (255, 255, 255, 255), 'bg': (32, 32, 32, 255)},
}


def font_for(text, px):
    path = LATIN if text.isascii() else CJK
    return ImageFont.truetype(path, px)


def render(text, size, fg, bg):
    """按目标尺寸原生渲染一张 RGBA"""
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    if bg:
        r = max(2, round(size * 0.22))
        d.rounded_rectangle((0, 0, size - 1, size - 1), radius=r, fill=bg)
        box = size * 0.68          # 有底色时字要缩一点，别顶到边
    else:
        box = size * 0.88

    # 二分找出刚好塞进 box 的字号：不同字号下字形高度不是线性的
    px, best = max(6, int(box), ), None
    for cand in range(max(6, int(size * 0.4)), size + 1):
        f = font_for(text, cand)
        l, t, r, b = d.textbbox((0, 0), text, font=f)
        if max(r - l, b - t) <= box:
            best, px = f, cand
        else:
            break
    f = best or font_for(text, px)

    l, t, r, b = d.textbbox((0, 0), text, font=f)
    d.text(((size - (r - l)) / 2 - l, (size - (b - t)) / 2 - t), text, font=f, fill=fg)
    return img


def ico_bytes(images):
    """把一组 RGBA 图拼成 .ico

    每张按 32 位 BMP 存：BITMAPINFOHEADER 的高度要写两倍（颜色区 + 掩码区），
    像素自下而上排；掩码全 0，但每行仍要补齐到 4 字节。
    """
    entries, blobs, offset = [], [], 6 + 16 * len(images)
    for img in images:
        w, h = img.size
        px = img.load()
        color = bytearray()
        for y in range(h - 1, -1, -1):
            for x in range(w):
                r, g, b, a = px[x, y]
                color += bytes((b, g, r, a))
        mask_row = (w + 31) // 32 * 4
        mask = bytes(mask_row * h)
        header = struct.pack('<IiiHHIIiiII', 40, w, h * 2, 1, 32, 0,
                             len(color) + len(mask), 0, 0, 0, 0)
        blob = header + bytes(color) + mask
        entries.append(struct.pack('<BBBBHHII', w % 256, h % 256, 0, 0, 1, 32,
                                   len(blob), offset))
        offset += len(blob)
        blobs.append(blob)
    return struct.pack('<HHH', 0, 1, len(images)) + b''.join(entries) + b''.join(blobs)


def main():
    os.makedirs(PNG_OUT, exist_ok=True)
    for set_id, style in SETS.items():
        d = os.path.join(ICO_OUT, set_id)
        os.makedirs(d, exist_ok=True)
        for state, text in STATES:
            imgs = [render(text, s, style['fg'], style['bg']) for s in SIZES]
            io.open(os.path.join(d, state + '.ico'), 'wb').write(ico_bytes(imgs))
            # 界面图库用的预览图，画到 3 倍再交给浏览器缩，高分屏上才不糊
            render(text, 96, style['fg'], style['bg']).save(
                os.path.join(PNG_OUT, f'{set_id}-{state}.png'), optimize=True)
        print(f'  {set_id:12} {len(STATES)} 个状态 × {len(SIZES)} 个尺寸')
    print(len(SETS), '套 ->', os.path.relpath(ICO_OUT, ROOT))


if __name__ == '__main__':
    if not os.path.exists(CJK):
        print('找不到微软雅黑，这个脚本只能在 Windows 上跑', file=sys.stderr)
        sys.exit(1)
    main()
