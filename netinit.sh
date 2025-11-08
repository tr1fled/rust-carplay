echo "Killing AP & Initializing NM"
sudo systemctl stop hostapd dhcpcd dnsmasq
sudo systemctl start NetworkManager