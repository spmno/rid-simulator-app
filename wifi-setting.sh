#!/bin/bash

# 自动检测系统类型并设置网卡名称
if grep -q "Raspberry Pi" /proc/device-tree/model 2>/dev/null || grep -q "raspi" /etc/os-release; then
    INTERFACE="wlan1"  # 树莓派系统使用wlan1
    echo "树莓派系统检测成功，使用接口: $INTERFACE"
elif [ -f /etc/os-release ] && grep -qi "ubuntu" /etc/os-release; then
    # 自动检测Ubuntu系统中的无线网卡名称
    INTERFACE=$(ifconfig -a | grep -oP '^wl\S+' | head -n1)
    
    if [ -z "$INTERFACE" ]; then
        echo "错误：未找到无线网络接口！"
        exit 1
    fi
    echo "Ubuntu系统检测成功，自动检测到接口: $INTERFACE"
else
    echo "错误：未知系统类型！"
    exit 1
fi

# 核心功能函数
configure_monitor_mode() {
    # 关闭网卡
    sudo ifconfig $INTERFACE down || {
        echo "错误：关闭网卡失败！";
        exit 1;
    }
    sleep 1

    # 设置监听模式
    sudo iwconfig $INTERFACE mode monitor || {
        echo "错误：设置监听模式失败！";
        sudo ifconfig $INTERFACE up;
        exit 1;
    }
    sleep 1

    # 启用网卡
    sudo ifconfig $INTERFACE up || {
        echo "错误：启用网卡失败！";
        exit 1;
    }
    sleep 1

    # 设置信道
    sudo iwconfig $INTERFACE channel 6 || {
        echo "错误：设置信道失败！";
        exit 1;
    }
    echo "网卡 $INTERFACE 已成功配置为监听模式（信道6）"
}

# 执行配置
configure_monitor_mode