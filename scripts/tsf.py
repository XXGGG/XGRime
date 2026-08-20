"""列出 / 切换 TSF 输入法配置文件（ITfInputProcessorProfiles）。

抓小狼毫候选框之前得先确认它是当前那个输入法；系统默认可能停在微软拼音上。
切完记得切回去，别把用户的输入法留在别的档上。
"""
import ctypes
import os
import sys
from ctypes import wintypes

import comtypes  # noqa: E402
from comtypes import GUID, COMMETHOD, IUnknown, HRESULT  # noqa: E402
import comtypes.client  # noqa: E402

CLSID_TF_InputProcessorProfiles = GUID('{33C53A50-F456-4884-B049-85FD643ECFED}')
IID_ITfInputProcessorProfiles = GUID('{1F02B6C5-7842-4EE6-8A0B-9A24183A95CA}')

LANGID = wintypes.WORD


class TF_LANGUAGEPROFILE(ctypes.Structure):
    _fields_ = [('clsid', GUID), ('langid', LANGID), ('catid', GUID),
                ('fActive', wintypes.BOOL), ('guidProfile', GUID)]


class IEnumTfLanguageProfiles(IUnknown):
    _iid_ = GUID('{3D61BF11-AC5F-42C8-A4CB-931BCC28C744}')
    _methods_ = [
        COMMETHOD([], HRESULT, 'Clone',
                  (['out'], ctypes.POINTER(ctypes.POINTER(IUnknown)), 'ppEnum')),
        COMMETHOD([], HRESULT, 'Next',
                  (['in'], wintypes.ULONG, 'ulCount'),
                  (['out'], ctypes.POINTER(TF_LANGUAGEPROFILE), 'pProfile'),
                  (['out'], ctypes.POINTER(wintypes.ULONG), 'pcFetch')),
        COMMETHOD([], HRESULT, 'Reset'),
        COMMETHOD([], HRESULT, 'Skip', (['in'], wintypes.ULONG, 'ulCount')),
    ]


class ITfInputProcessorProfiles(IUnknown):
    _iid_ = IID_ITfInputProcessorProfiles
    _methods_ = [
        COMMETHOD([], HRESULT, 'Register', (['in'], ctypes.POINTER(GUID), 'rclsid')),
        COMMETHOD([], HRESULT, 'Unregister', (['in'], ctypes.POINTER(GUID), 'rclsid')),
        COMMETHOD([], HRESULT, 'AddLanguageProfile'),
        COMMETHOD([], HRESULT, 'RemoveLanguageProfile'),
        COMMETHOD([], HRESULT, 'EnumInputProcessorInfo'),
        COMMETHOD([], HRESULT, 'GetDefaultLanguageProfile'),
        COMMETHOD([], HRESULT, 'SetDefaultLanguageProfile'),
        COMMETHOD([], HRESULT, 'ActivateLanguageProfile',
                  (['in'], ctypes.POINTER(GUID), 'rclsid'),
                  (['in'], LANGID, 'langid'),
                  (['in'], ctypes.POINTER(GUID), 'guidProfile')),
        COMMETHOD([], HRESULT, 'GetActiveLanguageProfile',
                  (['in'], ctypes.POINTER(GUID), 'rclsid'),
                  (['out'], ctypes.POINTER(LANGID), 'plangid'),
                  (['out'], ctypes.POINTER(GUID), 'pguidProfile')),
        COMMETHOD([], HRESULT, 'GetLanguageProfileDescription',
                  (['in'], ctypes.POINTER(GUID), 'rclsid'),
                  (['in'], LANGID, 'langid'),
                  (['in'], ctypes.POINTER(GUID), 'guidProfile'),
                  (['out'], ctypes.POINTER(comtypes.BSTR), 'pbstrProfile')),
        COMMETHOD([], HRESULT, 'GetCurrentLanguage',
                  (['out'], ctypes.POINTER(LANGID), 'plangid')),
        COMMETHOD([], HRESULT, 'ChangeCurrentLanguage', (['in'], LANGID, 'langid')),
        COMMETHOD([], HRESULT, 'GetLanguageList'),
        COMMETHOD([], HRESULT, 'EnumLanguageProfiles',
                  (['in'], LANGID, 'langid'),
                  (['out'], ctypes.POINTER(ctypes.POINTER(IEnumTfLanguageProfiles)), 'ppEnum')),
        COMMETHOD([], HRESULT, 'EnableLanguageProfile'),
        COMMETHOD([], HRESULT, 'IsEnabledLanguageProfile'),
        COMMETHOD([], HRESULT, 'EnableLanguageProfileByDefault'),
        COMMETHOD([], HRESULT, 'SubstituteKeyboardLayout'),
    ]


def profiles(langid=0x0804):
    p = comtypes.client.CreateObject(CLSID_TF_InputProcessorProfiles,
                                     interface=ITfInputProcessorProfiles)
    e = p.EnumLanguageProfiles(langid)
    out = []
    while True:
        try:
            prof, got = e.Next(1)
        except Exception:
            break
        if not got:
            break
        try:
            desc = p.GetLanguageProfileDescription(prof.clsid, prof.langid, prof.guidProfile)
        except Exception:
            desc = '?'
        out.append((desc, prof.clsid, prof.langid, prof.guidProfile, bool(prof.fActive)))
    return p, out


def activate(p, clsid, langid, guid):
    p.ActivateLanguageProfile(clsid, langid, guid)


if __name__ == '__main__':
    p, ps = profiles()
    for desc, clsid, langid, guid, active in ps:
        print(f'{"*" if active else " "} {desc}   {clsid}  {guid}')
