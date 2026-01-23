import React from 'react';
import { View, Text } from 'react-native';
import { NativeStackScreenProps } from '@react-navigation/native-stack';
import { RootStackParamList } from '../navigation/types';

type Props = NativeStackScreenProps<RootStackParamList, 'Profile'>;

export default function ProfileScreen({ route }: Props) {
    const { userId } = route.params;

    return (
        <View className="flex-1 items-center justify-center bg-white dark:bg-gray-900">
            <Text className="text-2xl font-bold text-gray-900 dark:text-white">
                Profile Screen
            </Text>
            <Text className="text-gray-600 dark:text-gray-300 mt-2">
                User ID: {userId}
            </Text>
        </View>
    );
}