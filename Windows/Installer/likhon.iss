; This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
; not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
; Likhon offline installer (Inno Setup 7). Milestone M8, Desktop/CLAUDE.md section 9.
;
; Build:  powershell -File Desktop\Installer\build-installer.ps1        (see that script for the options)
; Needs:  likhon.exe already built and signed (the exe carries the text-service DLLs inside it).
;         art\mark-*.bmp (node Desktop\Installer\art\make-art.mjs makes them from the brand feather).
;
; Look: the Likhon design system (light theme). Accent sidebar with the step list on the left, page on the right,
; a pill progress bar. Inno 7's built-in "windows11" custom style supplies the controls; everything else is drawn
; or laid out in [Code]. No image files are loaded at run time except the small brand mark.
;
; Rules this script keeps:
;  - Offline: it downloads nothing.
;  - File associations for layout and theme files are not offered: they are plain .json files, so there is no own extension to claim.
;  - Nothing is installed before the licence is accepted (the wizard does not copy files until the Install button).
;  - Default is "everyone on this PC" (C:\Program Files\Likhon, asks for permission once); "just me" needs no administrator permission.
;  - The same likhon.exe is the Portable app: copying it out of the install folder keeps working.
;  - The Windows keyboard list entry (Win+Space) is machine-wide, so it is only offered for "Everyone".
;    A "Just me" install can still add it later from Settings > General (one permission prompt).

#ifndef AppVersion
  #define AppVersion "0.2.1"
#endif
#ifndef ExePath
  #define ExePath "..\Portable\likhon.exe"
#endif
#ifndef DictDir
  #define DictDir "..\Portable\dict"
#endif
#ifndef OutDir
  #define OutDir "..\Setup"
#endif

[Setup]
AppId={{5B6C7D3E-1A2B-4C3D-8A3F-9E2D4C1B7A60}
AppName=Likhon
AppVersion={#AppVersion}
AppVerName=Likhon {#AppVersion}
AppPublisher=Agniflux Enterprise
AppPublisherURL=https://likhon.agniflux.com
AppSupportURL=https://likhon.agniflux.com/support
AppUpdatesURL=https://likhon.agniflux.com/download
VersionInfoVersion={#AppVersion}
DefaultDirName={autopf}\Likhon
DefaultGroupName=Likhon
DisableProgramGroupPage=yes
DisableWelcomePage=no
DisableReadyPage=yes
UninstallDisplayName=Likhon
UninstallDisplayIcon={app}\likhon.exe
SetupIconFile=..\..\Source\assets\icons\likhon.ico
OutputDir={#OutDir}
OutputBaseFilename=Likhon-Setup-{#AppVersion}
Compression=lzma2/ultra64
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
; Owner request 2026-11-21: the default folder is C:\Program Files\Likhon. Inno asks "Install for all users (recommended)
; or for me only?" first and raises the Windows prompt only if "all users" is chosen; "me only" needs no administrator
; permission and installs to %LOCALAPPDATA%\Programs\Likhon.
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=dialog
; Light only, on the design system's `bg` colour (#FBF9F4). hidebevels removes the grey divider lines.
WizardStyle=modern windows11 hidebevels
WizardBackColor=#FBF9F4
WizardImageFile=
WizardSmallImageFile=
; The agreement is the site's own text (Web/Laravel/resources/legal/*/license.md), one file per language: node license\make-eula.mjs
CloseApplications=yes
RestartApplications=no
SetupLogging=yes

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"; LicenseFile: "license\eula.en.txt"
; Bangla: Inno has no official Bangla file. Likhon's own texts come from [CustomMessages], and the wizard's standard
; texts are overridden in [Messages] below (all drafts, marked needs review, like the rest of the text library).
Name: "bn"; MessagesFile: "compiler:Default.isl"; LicenseFile: "license\eula.bn.txt"

[LangOptions]
en.LanguageName=English
bn.LanguageName=<09AC><09BE><0982><09B2><09BE>
bn.LanguageID=$0845

[Messages]
bn.SetupWindowTitle=ইনস্টল - %1
bn.SelectLanguageTitle=ভাষা বেছে নিন
bn.SelectLanguageLabel=ইনস্টলের সময় যে ভাষা ব্যবহার হবে সেটি বেছে নিন।
bn.ExitSetupTitle=ইনস্টল বন্ধ করুন
bn.ExitSetupMessage=ইনস্টল শেষ হয়নি। এখন বের হলে লিখন ইনস্টল হবে না।%n%nপরে আবার চালিয়ে ইনস্টল শেষ করতে পারেন।%n%nবের হবেন?
bn.ButtonBack=&পিছনে
bn.ButtonNext=&পরবর্তী
bn.ButtonInstall=&ইনস্টল
bn.ButtonOK=ঠিক আছে
bn.ButtonCancel=বাতিল
bn.ButtonYes=&হ্যাঁ
bn.ButtonNo=&না
bn.ButtonFinish=&শেষ
bn.ButtonBrowse=&খুঁজুন...
bn.WizardLicense=লাইসেন্স চুক্তি
bn.LicenseLabel=চালিয়ে যাওয়ার আগে অনুগ্রহ করে নিচের তথ্যটি পড়ুন।
bn.LicenseLabel3=অনুগ্রহ করে লাইসেন্স চুক্তিটি পড়ুন। ইনস্টল চালিয়ে যেতে চুক্তির শর্তগুলো মানতে হবে।
bn.LicenseAccepted=আমি চুক্তিটি &মেনে নিচ্ছি
bn.LicenseNotAccepted=আমি চুক্তিটি মানছি &না
bn.WizardSelectDir=ইনস্টলের জায়গা
bn.SelectDirDesc=লিখন কোথায় ইনস্টল হবে?
bn.SelectDirLabel3=লিখন নিচের ফোল্ডারে ইনস্টল হবে।
bn.SelectDirBrowseLabel=চালিয়ে যেতে পরবর্তী চাপুন। অন্য ফোল্ডার চাইলে খুঁজুন চাপুন।
bn.WizardSelectTasks=কী ইনস্টল হবে বেছে নিন
bn.SelectTasksDesc=পরে সেটিংসে সব বদলানো যাবে।
bn.SelectTasksLabel2=ইনস্টলের সময় যা যা হবে বেছে নিন, তারপর পরবর্তী চাপুন।
bn.WizardInstalling=ইনস্টল হচ্ছে
bn.InstallingLabel=লিখন ইনস্টল হচ্ছে, একটু অপেক্ষা করুন।
bn.StatusClosingApplications=চালু অ্যাপ বন্ধ করা হচ্ছে...
bn.StatusExtractFiles=ফাইল নামানো হচ্ছে...
bn.StatusCreateIcons=শর্টকাট তৈরি হচ্ছে...
bn.StatusCreateRegistryEntries=রেজিস্ট্রি এন্ট্রি তৈরি হচ্ছে...
bn.StatusSavingUninstall=আনইনস্টলের তথ্য রাখা হচ্ছে...
bn.StatusRunProgram=ইনস্টল শেষ হচ্ছে...
bn.StatusRollback=পরিবর্তন ফিরিয়ে নেওয়া হচ্ছে...

[CustomMessages]
en.StepWelcome=Welcome
en.StepLicense=License
en.StepOptions=Options
en.StepInstall=Installing
en.StepFinish=Finish
en.TitleWelcome=Install Likhon
en.DescWelcome=Free Bangla typing for Windows 10 and 11, version {#AppVersion}.%n%nThis installer works offline and downloads nothing. It installs for you, or for everyone on this PC if you choose that. Likhon does not record what you type.%n%nWant to try it first? The Portable file needs no install.
en.InstallNote=If you cancel part-way, everything already placed is removed and your PC goes back as it was.
en.TitleFinish=Likhon is ready
en.DescFinish=Press F12 in any app to switch between Bangla and English. Likhon lives in the tray: double-click its icon to open the top bar.
en.HelpLink=Help
en.TaskKeyboard=Show Likhon in the Windows keyboard list (Win+Space)
en.TaskKeyboardHint=Needs administrator permission, so it is only offered when installing for everyone. The text service reads your keystrokes only to turn them into Bangla.
en.TaskStartup=Start Likhon when I sign in to Windows
en.GroupExtras=Extras
en.FinishGuide=Open the Likhon first-run guide
en.AskKeepWords=Keep your personal words and settings?%n%nYes keeps them in your Likhon folder. No deletes them.
en.NoteNoWebView=The Settings window needs Microsoft WebView2. Typing works without it.
bn.StepWelcome=স্বাগতম
bn.StepLicense=লাইসেন্স
bn.StepOptions=বিকল্প
bn.StepInstall=ইনস্টল হচ্ছে
bn.StepFinish=শেষ
bn.TitleWelcome=লিখন ইনস্টল করুন
bn.DescWelcome=উইন্ডোজ ১০ ও ১১-এর জন্য বিনামূল্যের বাংলা টাইপিং, সংস্করণ {#AppVersion}।%n%nএই ইনস্টলার ইন্টারনেট ছাড়াই চলে, কিছু নামায় না। আপনার জন্য ইনস্টল হয়, চাইলে এই পিসির সবার জন্যও হয়। আপনি যা টাইপ করেন, লিখন তা রেকর্ড করে না।%n%nআগে একটু দেখে নিতে চান? পোর্টেবল ফাইলে কিছু ইনস্টল করতে হয় না।
bn.InstallNote=মাঝপথে বাতিল করলে যা বসানো হয়েছে সব সরে যাবে, আপনার পিসি আগের মতো থাকবে।
bn.TitleFinish=লিখন তৈরি
bn.DescFinish=যেকোনো অ্যাপে F12 চাপলে বাংলা ও English বদলাবে। লিখন ট্রেতে থাকে: টপ বার খুলতে এর আইকনে ডাবল-ক্লিক করুন।
bn.HelpLink=সাহায্য
bn.TaskKeyboard=উইন্ডোজের কিবোর্ড তালিকায় লিখন দেখান (Win+Space)
bn.TaskKeyboardHint=প্রশাসকের অনুমতি লাগে, তাই শুধু সবার জন্য ইনস্টল করলে দেওয়া হয়। টেক্সট সার্ভিস আপনার কিস্ট্রোক শুধু বাংলায় রূপান্তরের জন্য পড়ে।
bn.TaskStartup=উইন্ডোজে সাইন ইন করলে লিখন চালু করুন
bn.GroupExtras=অতিরিক্ত
bn.FinishGuide=লিখনের প্রথম ব্যবহারের গাইড খুলুন
bn.AskKeepWords=আপনার নিজের শব্দ ও সেটিংস রেখে দেবেন?%n%nহ্যাঁ হলে লিখন ফোল্ডারে থাকবে, না হলে মুছে যাবে।
bn.NoteNoWebView=সেটিংস উইন্ডোর জন্য Microsoft WebView2 লাগে। টাইপ করতে তা লাগে না।

[Tasks]
Name: "keyboard"; Description: "{cm:TaskKeyboard}"; GroupDescription: "{cm:GroupExtras}"; Check: IsAdminInstallMode
Name: "startup"; Description: "{cm:TaskStartup}"; GroupDescription: "{cm:GroupExtras}"; Flags: unchecked

[Files]
Source: "{#ExePath}"; DestDir: "{app}"; DestName: "likhon.exe"; Flags: ignoreversion
; The system dictionary (read-only SQLite, MPL-2.0) with its licence and source note, beside the exe like Portable.
Source: "{#DictDir}\*"; DestDir: "{app}\dict"; Flags: ignoreversion
; The MPL-2.0 notices and the source offer, beside the exe (License notices, 2026-11-21).
Source: "..\..\NOTICE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\NOTICES.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\SOURCE-OFFER.txt"; DestDir: "{app}"; Flags: ignoreversion
; Installer window only (never installed): the brand mark, and Hind Siliguri (OFL 1.1) loaded privately for this window.
Source: "art\mark-44.bmp"; Flags: dontcopy
Source: "art\mark-66.bmp"; Flags: dontcopy
Source: "art\mark-88.bmp"; Flags: dontcopy
Source: "art\splash-464.bmp"; Flags: dontcopy
Source: "art\splash-696.bmp"; Flags: dontcopy
Source: "art\splash-928.bmp"; Flags: dontcopy
Source: "..\..\Source\assets\fonts\HindSiliguri-Regular.ttf"; Flags: dontcopy
Source: "..\..\Source\assets\fonts\HindSiliguri-Bold.ttf"; Flags: dontcopy

[Icons]
Name: "{autoprograms}\Likhon"; Filename: "{app}\likhon.exe"

[Registry]
; Start with Windows: this user, or everyone when installed for everyone. Same value name the app itself uses.
Root: HKA; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Likhon"; ValueData: """{app}\likhon.exe"""; Tasks: startup; Flags: uninsdeletevalue

[Run]
; Windows keyboard list: the exe carries both text-service DLLs and registers them (machine-wide, so admin install only).
Filename: "{app}\likhon.exe"; Parameters: "--keyboard-add ""{localappdata}\Likhon"""; Tasks: keyboard; Flags: runhidden waituntilterminated
; The installer showed the agreement and it was accepted: record that for this user, so the first start does not ask again.
Filename: "{app}\likhon.exe"; Parameters: "--accept-agreement"; Flags: runhidden waituntilterminated runasoriginaluser
; Finish page: start Likhon, and offer the first-run guide (the Settings window's guide page).
Filename: "{app}\likhon.exe"; Description: "{cm:LaunchProgram,Likhon}"; Flags: nowait postinstall skipifsilent
Filename: "{app}\likhon.exe"; Parameters: "--settings general"; Description: "{cm:FinishGuide}"; Flags: nowait postinstall skipifsilent unchecked

[UninstallRun]
; Take the keyboard out of Windows first (a no-op when it was never added). Must run before the exe is deleted.
Filename: "{app}\likhon.exe"; Parameters: "--keyboard-remove"; RunOnceId: "KeyboardRemove"; Flags: runhidden waituntilterminated; Check: IsAdminInstallMode

[Code]
{ ---------- Likhon design system colours (Inno wants BGR: $BBGGRR) ---------- }
const
  CLR_ACCENT  = $C44727;  { #2747C4 accent: sidebar, primary }
  CLR_ON      = $FFFFFF;  { #FFFFFF on: text on accent }
  CLR_PENDING = $F3D2C9;  { #C9D2F3 muted text on accent (about 5:1) }
  CLR_BG      = $F4F9FB;  { #FBF9F4 bg }
  CLR_SURFACE2= $EAF1F4;  { #F4F1EA surface2: progress track }
  CLR_INK     = $30221B;  { #1B2230 ink }
  CLR_MUTED   = $74645B;  { #5B6474 muted }

  SIDEBAR_W = 232;        { design pixels at 100% }
  STEP_COUNT = 5;
  BAR_H = 8;

function AddFontResourceEx(lpFilename: String; fl: DWORD; pdv: Integer): Integer;
  external 'AddFontResourceExW@gdi32.dll stdcall';

var
  Sidebar: TBitmapImage;
  MarkImage: TBitmapImage;
  BrandLabel, TitleLabel, DescLabel, HelpLabel, WebViewNote, InstallNote: TNewStaticText;
  SplashImage: TBitmapImage;
  StepLabel: array[0..STEP_COUNT - 1] of TNewStaticText;
  StepIcon: array[0..STEP_COUNT - 1] of TBitmapImage;
  StateBmp: array[0..2] of TBitmap;   { 0 pending, 1 current, 2 done }
  BarImage: TBitmapImage;
  BarW, CapR: Integer;
  CapCov: array of Integer;           { 0..16 coverage of the left cap, CapR x BAR_H }
  BodyFont, HeadFont: String;
  FontsLoaded: Boolean;

function Blend(Fg, Bg, Alpha: Integer): Integer;  { Alpha 0..255 of Fg over Bg }
var
  r, g, b: Integer;
begin
  r := ((Fg and $FF) * Alpha + (Bg and $FF) * (255 - Alpha)) div 255;
  g := (((Fg shr 8) and $FF) * Alpha + ((Bg shr 8) and $FF) * (255 - Alpha)) div 255;
  b := (((Fg shr 16) and $FF) * Alpha + ((Bg shr 16) and $FF) * (255 - Alpha)) div 255;
  Result := r or (g shl 8) or (b shl 16);
end;

{ ---------- fonts: Hind Siliguri, loaded for this process only ---------- }
procedure LoadBrandFonts;
begin
  BodyFont := 'Segoe UI';
  HeadFont := 'Segoe UI';
  try
    ExtractTemporaryFile('HindSiliguri-Regular.ttf');
    ExtractTemporaryFile('HindSiliguri-Bold.ttf');
    if (AddFontResourceEx(ExpandConstant('{tmp}\HindSiliguri-Regular.ttf'), $10, 0) > 0) and
       (AddFontResourceEx(ExpandConstant('{tmp}\HindSiliguri-Bold.ttf'), $10, 0) > 0) then
    begin
      BodyFont := 'Hind Siliguri';
      HeadFont := 'Hind Siliguri';
      FontsLoaded := True;
    end;
  except
    { keep Segoe UI: Windows falls back to Nirmala UI for Bangla }
  end;
end;

{ ---------- small anti-aliased drawing (4 x 4 samples per pixel) ---------- }
function SegDistSq(px, py, ax, ay, bx, by: Double): Double;
var
  dx, dy, t, qx, qy: Double;
begin
  dx := bx - ax; dy := by - ay;
  t := ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy);
  if t < 0 then t := 0;
  if t > 1 then t := 1;
  qx := ax + t * dx - px; qy := ay + t * dy - py;
  Result := qx * qx + qy * qy;
end;

{ Step marker: pending = faint disc, current = white disc with an accent dot, done = faint disc with a white tick. }
procedure DrawMarker(Bmp: TBitmap; State, Size: Integer);
var
  x, y, sx, sy, hit, hitTick, hitDot: Integer;
  u, v, c, r, rr, dotR, half, col: Double;
  px, py: Integer;
  fillColor, color: Integer;
begin
  Bmp.Width := Size; Bmp.Height := Size;
  c := Size / 2; r := Size / 2 - 0.5; rr := r * r; dotR := Size * 0.17; half := Size * 0.055;
  if State = 1 then fillColor := CLR_ON
  else if State = 2 then fillColor := Blend(CLR_ON, CLR_ACCENT, 75)
  else fillColor := Blend(CLR_ON, CLR_ACCENT, 56);
  for y := 0 to Size - 1 do
    for x := 0 to Size - 1 do
    begin
      hit := 0; hitTick := 0; hitDot := 0;
      for sy := 0 to 3 do
        for sx := 0 to 3 do
        begin
          u := x + (sx + 0.5) / 4; v := y + (sy + 0.5) / 4;
          if (u - c) * (u - c) + (v - c) * (v - c) <= rr then
          begin
            hit := hit + 1;
            if State = 1 then
            begin
              if (u - c) * (u - c) + (v - c) * (v - c) <= dotR * dotR then hitDot := hitDot + 1;
            end
            else if State = 2 then
            begin
              if (SegDistSq(u, v, Size * 0.29, Size * 0.52, Size * 0.44, Size * 0.67) <= half * half) or
                 (SegDistSq(u, v, Size * 0.44, Size * 0.67, Size * 0.73, Size * 0.35) <= half * half) then
                hitTick := hitTick + 1;
            end;
          end;
        end;
      color := Blend(fillColor, CLR_ACCENT, hit * 255 div 16);
      if hitDot > 0 then color := Blend(CLR_ACCENT, color, hitDot * 255 div 16);
      if hitTick > 0 then color := Blend(CLR_ON, color, hitTick * 255 div 16);
      Bmp.Canvas.Pixels[x, y] := color;
    end;
end;

{ Progress bar: a pill, track in surface2, fill in accent. The caps are anti-aliased once; the middle is solid columns. }
procedure PrepareBarCap;
var
  x, y, sx, sy, hit: Integer;
  u, v, r: Double;
begin
  CapR := BAR_H div 2;
  r := CapR;
  SetArrayLength(CapCov, CapR * BAR_H);
  for x := 0 to CapR - 1 do
    for y := 0 to BAR_H - 1 do
    begin
      hit := 0;
      for sy := 0 to 3 do
        for sx := 0 to 3 do
        begin
          u := x + (sx + 0.5) / 4; v := y + (sy + 0.5) / 4;
          if (u - r) * (u - r) + (v - r) * (v - r) <= r * r then hit := hit + 1;
        end;
      CapCov[x * BAR_H + y] := hit;
    end;
end;

procedure DrawBar(FillW: Integer);
var
  x, y, cov: Integer;
  col: Integer;
begin
  if BarImage = nil then Exit;
  for x := 0 to BarW - 1 do
  begin
    if x < FillW then col := CLR_ACCENT else col := CLR_SURFACE2;
    if (x < CapR) or (x >= BarW - CapR) then
    begin
      for y := 0 to BAR_H - 1 do
      begin
        if x < CapR then cov := CapCov[x * BAR_H + y]
        else cov := CapCov[(BarW - 1 - x) * BAR_H + y];
        BarImage.Bitmap.Canvas.Pixels[x, y] := Blend(col, CLR_BG, cov * 255 div 16);
      end;
    end
    else
    begin
      BarImage.Bitmap.Canvas.Pen.Color := col;
      BarImage.Bitmap.Canvas.MoveTo(x, 0);
      BarImage.Bitmap.Canvas.LineTo(x, BAR_H);
    end;
  end;
  BarImage.Repaint;
end;

procedure CurInstallProgressChanged(CurProgress, MaxProgress: Integer);
begin
  if MaxProgress > 0 then DrawBar(Int64(BarW) * CurProgress div MaxProgress);
end;

{ ---------- layout ---------- }
function StepIndexFor(PageID: Integer): Integer;
begin
  case PageID of
    wpWelcome: Result := 0;
    wpLicense, wpInfoBefore: Result := 1;
    wpPreparing, wpInstalling: Result := 3;
    wpInfoAfter, wpFinished: Result := 4;
  else
    Result := 2;  { folder, features }
  end;
end;

procedure HelpClick(Sender: TObject);
var
  rc: Integer;
begin
  ShellExec('open', 'https://likhon.agniflux.com/support', '', '', SW_SHOWNORMAL, ewNoWait, rc);
end;

function NewLabel(AParent: TWinControl; const Cap: String; ALeft, ATop, AWidth, AHeight, SizePx: Integer; Bold: Boolean; Clr, BgClr: Integer): TNewStaticText;
begin
  Result := TNewStaticText.Create(WizardForm);
  Result.Parent := AParent;
  Result.AutoSize := False;
  Result.WordWrap := True;
  Result.SetBounds(ScaleX(ALeft), ScaleY(ATop), ScaleX(AWidth), ScaleY(AHeight));
  if Bold then Result.Font.Name := HeadFont else Result.Font.Name := BodyFont;
  Result.Font.Height := -ScaleY(SizePx);
  if Bold then Result.Font.Style := [fsBold];
  Result.StyleElements := [];
  Result.Color := BgClr;
  Result.Font.Color := Clr;
  Result.Caption := Cap;
end;

procedure PlaceMain(Ctl: TControl; ALeft, ATop, AWidth, AHeight: Integer);
begin
  Ctl.SetBounds(ScaleX(ALeft), ScaleY(ATop), ScaleX(AWidth), ScaleY(AHeight));
end;

procedure InitializeWizard;
var
  i, markSize, icon: Integer;
  stepNames: array[0..STEP_COUNT - 1] of String;
  mark, splash: String;
begin
  LoadBrandFonts;
  PrepareBarCap;

  WizardForm.Font.Name := BodyFont;
  WizardForm.Font.Height := -ScaleY(13);
  WizardForm.ClientWidth := ScaleX(760);
  WizardForm.ClientHeight := ScaleY(480);

  { Hide the default header and the big picture; the page title comes from our own labels. }
  WizardForm.MainPanel.Visible := False;
  WizardForm.WizardBitmapImage.Visible := False;
  WizardForm.WizardBitmapImage2.Visible := False;
  WizardForm.WizardSmallBitmapImage.Visible := False;
  WizardForm.WelcomeLabel1.Visible := False;
  WizardForm.FinishedHeadingLabel.Visible := False;

  { Sidebar }
  Sidebar := TBitmapImage.Create(WizardForm);
  Sidebar.Parent := WizardForm;
  Sidebar.AutoSize := False;
  Sidebar.Stretch := True;
  Sidebar.Bitmap.Width := 1;
  Sidebar.Bitmap.Height := 1;
  Sidebar.Bitmap.Canvas.Pixels[0, 0] := CLR_ACCENT;
  Sidebar.SetBounds(0, 0, ScaleX(SIDEBAR_W), WizardForm.ClientHeight);
  Sidebar.SendToBack;

  markSize := ScaleX(44);
  if markSize > 60 then mark := 'mark-66.bmp' else mark := 'mark-44.bmp';
  if markSize > 80 then mark := 'mark-88.bmp';
  ExtractTemporaryFile(mark);
  MarkImage := TBitmapImage.Create(WizardForm);
  MarkImage.Parent := WizardForm;
  MarkImage.AutoSize := True;
  MarkImage.Bitmap.LoadFromFile(ExpandConstant('{tmp}\' + mark));
  MarkImage.SetBounds(ScaleX(28), ScaleY(26), MarkImage.Bitmap.Width, MarkImage.Bitmap.Height);
  BrandLabel := NewLabel(WizardForm, 'Likhon · লিখন', 80, 36, 140, 28, 20, True, CLR_ON, CLR_ACCENT);

  { Step list }
  stepNames[0] := CustomMessage('StepWelcome');
  stepNames[1] := CustomMessage('StepLicense');
  stepNames[2] := CustomMessage('StepOptions');
  stepNames[3] := CustomMessage('StepInstall');
  stepNames[4] := CustomMessage('StepFinish');
  for i := 0 to 2 do
  begin
    StateBmp[i] := TBitmap.Create;
    DrawMarker(StateBmp[i], i, ScaleX(24));
  end;
  for i := 0 to STEP_COUNT - 1 do
  begin
    StepIcon[i] := TBitmapImage.Create(WizardForm);
    StepIcon[i].Parent := WizardForm;
    StepIcon[i].AutoSize := True;
    StepIcon[i].Bitmap.Assign(StateBmp[0]);
    StepIcon[i].SetBounds(ScaleX(32), ScaleY(124 + i * 44), StateBmp[0].Width, StateBmp[0].Height);
    StepLabel[i] := NewLabel(WizardForm, stepNames[i], 68, 124 + i * 44 + 1, 150, 24, 15, False, CLR_PENDING, CLR_ACCENT);
  end;

  HelpLabel := NewLabel(WizardForm, CustomMessage('HelpLink'), 32, 436, 120, 22, 14, False, CLR_ON, CLR_ACCENT);
  HelpLabel.Font.Style := [fsUnderline];
  HelpLabel.Cursor := crHand;
  HelpLabel.OnClick := @HelpClick;

  { Page title and description on the right }
  TitleLabel := NewLabel(WizardForm, '', 264, 28, 464, 34, 24, True, CLR_INK, CLR_BG);
  DescLabel := NewLabel(WizardForm, '', 264, 66, 464, 40, 14, False, CLR_MUTED, CLR_BG);

  { Content zone: x 264..728, y 116..408. Buttons stay where Inno anchors them (bottom right). }
  PlaceMain(WizardForm.OuterNotebook, 264, 116, 464, 292);
  PlaceMain(WizardForm.InnerNotebook, 0, 0, 464, 292);
  PlaceMain(WizardForm.WelcomeLabel2, 0, 0, 464, 200);
  PlaceMain(WizardForm.FinishedLabel, 0, 0, 464, 80);
  WizardForm.RunList.Left := 0;
  WizardForm.RunList.Width := ScaleX(464);
  PlaceMain(WizardForm.WelcomeLabel2, 0, 0, 464, 138);
  WizardForm.WelcomeLabel2.Font.Height := -ScaleY(13);
  WizardForm.FinishedLabel.Font.Height := -ScaleY(14);

  { A banner cropped from the splash artwork (rounded corners are baked in, in the page colour). }
  if ScaleX(464) > 600 then splash := 'splash-696.bmp' else splash := 'splash-464.bmp';
  if ScaleX(464) > 800 then splash := 'splash-928.bmp';
  ExtractTemporaryFile(splash);
  SplashImage := TBitmapImage.Create(WizardForm);
  SplashImage.Parent := WizardForm.WelcomePage;
  SplashImage.AutoSize := True;
  SplashImage.Bitmap.LoadFromFile(ExpandConstant('{tmp}\' + splash));
  SplashImage.SetBounds(0, ScaleY(146), SplashImage.Bitmap.Width, SplashImage.Bitmap.Height);

  InstallNote := NewLabel(WizardForm.InstallingPage, CustomMessage('InstallNote'), 0, 84, 464, 40, 13, False, CLR_MUTED, CLR_BG);

  { Licence page: label, text box, then the two choices, all inside the 464 x 292 zone. }
  PlaceMain(WizardForm.LicenseLabel1, 0, 0, 464, 44);
  PlaceMain(WizardForm.LicenseMemo, 0, 48, 464, 184);
  PlaceMain(WizardForm.LicenseAcceptedRadio, 0, 240, 464, 22);
  PlaceMain(WizardForm.LicenseNotAcceptedRadio, 0, 264, 464, 22);

  { Installing page: our pill bar sits where the stock gauge is. }
  WizardForm.ProgressGauge.Visible := False;
  BarW := ScaleX(464);
  BarImage := TBitmapImage.Create(WizardForm);
  BarImage.Parent := WizardForm.InstallingPage;
  BarImage.AutoSize := False;
  BarImage.Stretch := False;
  BarImage.Bitmap.Width := BarW;
  BarImage.Bitmap.Height := BAR_H;
  BarImage.SetBounds(0, ScaleY(52), BarW, BAR_H);
  CapR := BAR_H div 2;
  DrawBar(0);

  if not (RegValueExists(HKLM, 'SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv') or
          RegValueExists(HKCU, 'Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv')) then
  begin
    { A plain note, never a download (the installer is offline): typing never needs WebView2, only Settings does. }
    WebViewNote := NewLabel(WizardForm.WelcomePage, CustomMessage('NoteNoWebView'), 0, 272, 464, 20, 12, False, CLR_MUTED, CLR_BG);
  end;
end;

procedure UpdateSteps(Step: Integer; AllDone: Boolean);
var
  i: Integer;
begin
  for i := 0 to STEP_COUNT - 1 do
  begin
    if AllDone or (i < Step) then
    begin
      StepIcon[i].Bitmap.Assign(StateBmp[2]);
      StepLabel[i].Font.Color := CLR_ON;
      StepLabel[i].Font.Style := [];
    end
    else if i = Step then
    begin
      StepIcon[i].Bitmap.Assign(StateBmp[1]);
      StepLabel[i].Font.Color := CLR_ON;
      StepLabel[i].Font.Style := [fsBold];
    end
    else
    begin
      StepIcon[i].Bitmap.Assign(StateBmp[0]);
      StepLabel[i].Font.Color := CLR_PENDING;
      StepLabel[i].Font.Style := [];
    end;
  end;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  UpdateSteps(StepIndexFor(CurPageID), CurPageID = wpFinished);
  case CurPageID of
    wpWelcome:
      begin
        TitleLabel.Caption := CustomMessage('TitleWelcome');
        DescLabel.Caption := '';
        WizardForm.WelcomeLabel2.Caption := CustomMessage('DescWelcome');
      end;
    wpFinished:
      begin
        TitleLabel.Caption := CustomMessage('TitleFinish');
        DescLabel.Caption := '';
        WizardForm.FinishedLabel.Caption := CustomMessage('DescFinish');
      end;
  else
    TitleLabel.Caption := WizardForm.PageNameLabel.Caption;
    DescLabel.Caption := WizardForm.PageDescriptionLabel.Caption;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  data: String;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    // Personal words and settings live beside the exe (when the folder is writable) or in %LOCALAPPDATA%\Likhon.
    // Ask before removing them. Copies used as Portable elsewhere are never touched.
    if UninstallSilent then Exit; // silent uninstall keeps the person's words
    data := ExpandConstant('{localappdata}\Likhon');
    if DirExists(data) then
      if MsgBox(CustomMessage('AskKeepWords'), mbConfirmation, MB_YESNO or MB_DEFBUTTON1) = IDNO then
        DelTree(data, True, True, True);
  end;
end;

