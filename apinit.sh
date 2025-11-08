echo "Killing NM & Initializing AP"
sudo systemctl stop NetworkManager
sudo systemctl start hostapd dhcpcd dnsmasq