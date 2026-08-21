Name:           protonsearch-linux
Version:        1.0.0
Release:        1%{?dist}
Summary:        Linux-native ProtonSearch launcher
License:        MIT
URL:            https://github.com/PranshulSoni/protonsearch
BuildArch:      x86_64
Requires:       gtk4
Requires:       gdk-pixbuf2
Suggests:        wl-clipboard
Suggests:        xclip
Suggests:        grim
Suggests:        slurp
Suggests:        poppler-utils
Suggests:        NetworkManager
Suggests:        bluez
Suggests:        wireplumber
Suggests:        brightnessctl
Suggests:        playerctl
Suggests:        power-profiles-daemon
Suggests:        upower

%description
Linux-native ProtonSearch launcher and search providers with runtime
distribution, desktop, display-server, and capability detection.

%install
install -Dpm0755 %{_sourcedir}/protonsearch-linux %{buildroot}%{_bindir}/protonsearch-linux
install -Dpm0644 %{_sourcedir}/protonsearch-linux.desktop %{buildroot}%{_datadir}/applications/protonsearch-linux.desktop
install -Dpm0644 %{_sourcedir}/protonsearch.png %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/protonsearch.png
install -Dpm0644 %{_sourcedir}/protonsearch-128.png %{buildroot}%{_datadir}/icons/hicolor/128x128/apps/protonsearch.png
install -Dpm0644 %{_sourcedir}/protonsearch.service %{buildroot}%{_userunitdir}/protonsearch.service

%files
%{_bindir}/protonsearch-linux
%{_datadir}/applications/protonsearch-linux.desktop
%{_datadir}/icons/hicolor/256x256/apps/protonsearch.png
%{_datadir}/icons/hicolor/128x128/apps/protonsearch.png
%{_userunitdir}/protonsearch.service

%changelog
* Thu Jan 01 1970 ProtonSearch contributors - 1.0.0-1
- Add distribution-aware Linux launcher package.
