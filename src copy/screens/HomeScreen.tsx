import React from 'react';
import { View, Text, TouchableOpacity } from 'react-native';
import { NativeStackScreenProps } from '@react-navigation/native-stack';
import { RootStackParamList } from '../navigation/types';

type Props = NativeStackScreenProps<RootStackParamList, 'Home'>;

export default function HomeScreen({ navigation }: Props) {
    return (
        <View className="flex-1 items-center justify-center bg-white dark:bg-gray-900">
            <Text className="text-3xl font-bold text-gray-900 dark:text-white mb-4">
                Welcome to TeachLink
            </Text>
            <Text className="text-gray-600 dark:text-gray-300 mb-8 px-4 text-center">
                Share and consume knowledge on the go
            </Text>

            <TouchableOpacity
                className="bg-blue-600 px-6 py-3 rounded-lg"
                onPress={() => navigation.navigate('Profile', { userId: '123' })}
            >
                <Text className="text-white font-semibold">Go to Profile</Text>
            </TouchableOpacity>

            <TouchableOpacity
                className="bg-gray-200 dark:bg-gray-700 px-6 py-3 rounded-lg mt-4"
                onPress={() => navigation.navigate('Settings')}
            >
                <Text className="text-gray-900 dark:text-white font-semibold">Settings</Text>
            </TouchableOpacity>
        </View>
    );
}