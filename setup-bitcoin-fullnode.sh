#!/bin/bash

# btc script

Crontab_file="/usr/bin/crontab"
Green_font_prefix="\033[32m"
Red_font_prefix="\033[31m"
Green_background_prefix="\033[42;37m"
Red_background_prefix="\033[41;37m"
Font_color_suffix="\033[0m"
Info="[${Green_font_prefix}Info${Font_color_suffix}]"
Error="[${Red_font_prefix}Error${Font_color_suffix}]"
Tip="[${Green_font_prefix}Warning${Font_color_suffix}]"
disk_info=$(df -h | grep -E '^/dev/' | sort -k4 -h -r)
max_disk=$(echo "$disk_info" | head -n 1 | awk '{print $1}')
max_disk_path=$(echo "$disk_info" | head -n 1 | awk '{print $6}')
cd "$max_disk_path" || exit

check_root() {
    [[ $EUID != 0 ]] && echo -e "${Error} The current user is not ROOT (or does not have ROOT privileges), unable to continue, please switch to the ROOT account or use ${Green_background_prefix}sudo su${Font_color_suffix} command to get temporary ROOT privileges (you may be prompted to enter the current user's password)." && exit 1
}

install_btc_full_node() {
    check_root
    sudo apt update && sudo apt upgrade -y
    sudo apt install curl tar wget clang pkg-config libssl-dev jq build-essential git make ncdu unzip zip -y
    sudo ufw allow 8333

    echo "The path of the largest disk is: $max_disk_path"

    latest_version_info=$(curl -s https://api.github.com/repos/bitcoin/bitcoin/releases/latest | grep "tag_name" | cut -d'"' -f4)
    latest_version=${latest_version_info#v}
    download_link="https://bitcoincore.org/bin/bitcoin-core-$latest_version/bitcoin-$latest_version-x86_64-linux-gnu.tar.gz"
    bitcoin_coin_path="$max_disk_path/bitcoin-core.tar.gz"

    wget -O $bitcoin_coin_path $download_link && \
    tar -xvf $bitcoin_coin_path && \
    bitcoin_directory=$(tar -tf $bitcoin_coin_path | head -n 1 | cut -f1 -d'/') && \
    mv "$bitcoin_directory" bitcoin-core && \
    chmod +x bitcoin-core

    echo "# Bitcoin environment variables" >> ~/.bashrc
    echo "export BTCPATH=$max_disk_path/bitcoin-core/bin" >> ~/.bashrc
    echo 'export PATH=$BTCPATH:$PATH' >> ~/.bashrc

    mkdir $max_disk_path/btc-data

    conf_file="$max_disk_path/btc-data/bitcoin.conf"
    conf_content=$(cat <<EOL
server=1
daemon=1
txindex=1
dbcache=4092
rpcallowip=0.0.0.0/0
rpcuser=your-rpc-user
rpcpassword=your-rpc-password
EOL
)
    # Check if the configuration file already exists, create it if it does not exist
    if [ ! -f "$conf_file" ]; then
        echo "$conf_content" > "$conf_file"
        echo "bitcoin.conf created at $conf_file"
    else
        echo "bitcoin.conf already exists at $conf_file"
    fi

    source ~/.bashrc
}

run_btc_full_node() {
    source ~/.bashrc
    bitcoin-cli -datadir=$max_disk_path/btc-data stop > /dev/null 2>&1
    bitcoind -datadir=$max_disk_path/btc-data -txindex
}

check_btc_full_node_block_height() {
    source ~/.bashrc
    bitcoin-cli -rpcuser=your-rpc-user -rpcpassword=your-rpc-password getblockchaininfo
}

check_btc_full_node_log() {
    source ~/.bashrc
    tail -f $max_disk_path/btc-data/debug.log
}

echo && echo -e " ${Red_font_prefix}dusk_network One-click Installation Script${Font_color_suffix} by \033[1;35moooooyoung\033[0m
This script is completely free and open source, developed by Twitter user ${Green_font_prefix}@ouyoung11${Font_color_suffix} forked by Purson.
 ———————————————————————
 ${Green_font_prefix} 1. Install Btc full node environment ${Font_color_suffix}
 ${Green_font_prefix} 2. Run Btc full node ${Font_color_suffix}
 ${Green_font_prefix} 3. Check Btc full node block height ${Font_color_suffix}
 ${Green_font_prefix} 4. Check Btc full node logs ${Font_color_suffix}
 ———————————————————————" && echo
read -e -p " Please follow the steps above and enter the number:" num
case "$num" in
1)
    install_btc_full_node
    ;;
2)
    run_btc_full_node
    ;;
3)
    check_btc_full_node_block_height
    ;;
4)
    check_btc_full_node_log
    ;;
*)
    echo
    echo -e " ${Error} Please enter the correct number"
    ;;
esac
