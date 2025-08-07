#!/bin/bash

# 获取所有可用的WiFi接口
get_available_interfaces() {
    # 查找所有以wl开头的接口 (wlan, wlp, wlx等)
    interfaces=$(ifconfig -a 2>/dev/null | grep -oP '^wl\S+' || ip link show 2>/dev/null | grep -oP '^\d+: \Kwl\S+(?=:)' || echo "")
    
    if [ -z "$interfaces" ]; then
        # 尝试使用iwconfig查找无线接口
        interfaces=$(iwconfig 2>/dev/null | grep -oP '^\S+(?=.*IEEE 802.11)' || echo "")
    fi
    
    echo "$interfaces"
}

# 显示可用接口并让用户选择
select_interface() {
    available_interfaces=$(get_available_interfaces)
    
    if [ -z "$available_interfaces" ]; then
        echo "错误：未找到无线网络接口！"
        exit 1
    fi
    
    # 将接口转换为数组
    interfaces_array=($available_interfaces)
    count=${#interfaces_array[@]}
    
    if [ $count -eq 1 ]; then
        INTERFACE=${interfaces_array[0]}
        echo "检测到单个接口: $INTERFACE"
    else
        echo "检测到多个无线网络接口:"
        for i in $(seq 1 $count); do
            echo "  $i) ${interfaces_array[$((i-1))]}"
        done
        
        # 默认选择第一个接口
        default_choice=1
        read -p "请选择要配置的接口 (1-$count) [$default_choice]: " choice
        
        # 如果用户直接回车，使用默认值
        if [ -z "$choice" ]; then
            choice=$default_choice
        fi
        
        # 验证选择
        if ! [[ "$choice" =~ ^[0-9]+$ ]] || [ "$choice" -lt 1 ] || [ "$choice" -gt "$count" ]; then
            echo "错误：无效的选择！"
            exit 1
        fi
        
        INTERFACE=${interfaces_array[$((choice-1))]}
    fi
    
    echo "将配置接口: $INTERFACE"
}

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

# 主程序
main() {
    echo "=== RID Simulator WiFi配置工具 ==="
    echo "检测可用网络接口..."
    
    select_interface
    configure_monitor_mode
}

# 执行主程序
main "$@"