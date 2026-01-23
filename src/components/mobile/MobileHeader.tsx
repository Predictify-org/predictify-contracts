import React from 'react';
import { View, Text, TouchableOpacity } from 'react-native';
import { useSafeArea } from '../../hooks/useSafeArea';
import { Menu, ArrowLeft, Bell } from 'lucide-react-native';
import { useNavigation } from '@react-navigation/native';
import { DrawerNavigationProp } from '@react-navigation/drawer';

interface MobileHeaderProps {
    title: string;
    showBack?: boolean;
    rightAction?: React.ReactNode;
}

export const MobileHeader = ({ title, showBack = false, rightAction }: MobileHeaderProps) => {
    const { top } = useSafeArea();
    const navigation = useNavigation<DrawerNavigationProp<any>>();

    return (
        <View
            className="bg-white border-b border-gray-200 flex-row items-center justify-between px-4 pb-3"
            style={{ paddingTop: top }}
        >
            <View className="flex-row items-center gap-3">
                {showBack ? (
                    <TouchableOpacity onPress={() => navigation.goBack()} className="p-2">
                        <ArrowLeft color="#1F2937" size={24} />
                    </TouchableOpacity>
                ) : (
                    <TouchableOpacity onPress={() => navigation.openDrawer()} className="p-2">
                        <Menu color="#1F2937" size={24} />
                    </TouchableOpacity>
                )}
                <Text className="text-lg font-bold text-gray-900">{title}</Text>
            </View>

            <View className="flex-row items-center">
                {rightAction || (
                    <TouchableOpacity className="p-2">
                        <Bell color="#4B5563" size={20} />
                    </TouchableOpacity>
                )}
            </View>
        </View>
    );
};
