import React from 'react';
<<<<<<< HEAD
import { View, Text, TouchableOpacity, ScrollView } from 'react-native';
import { NativeStackScreenProps } from '@react-navigation/native-stack';
import { RootStackParamList } from '../navigation/types';
import { sampleCourse } from '../data/sampleCourse';
=======
import { View, Text, TouchableOpacity } from 'react-native';
import { NativeStackScreenProps } from '@react-navigation/native-stack';
import { RootStackParamList } from '../navigation/types';
>>>>>>> b932655445289cc6885ffad4b922c05b464845b2

type Props = NativeStackScreenProps<RootStackParamList, 'Home'>;

export default function HomeScreen({ navigation }: Props) {
    return (
<<<<<<< HEAD
        <ScrollView
            className="flex-1 bg-white dark:bg-gray-900"
            contentContainerStyle={{ flexGrow: 1, alignItems: 'center', justifyContent: 'center', padding: 24 }}
        >
            <Text className="text-3xl font-bold text-gray-900 dark:text-white mb-4 text-center">
=======
        <View className="flex-1 items-center justify-center bg-white dark:bg-gray-900">
            <Text className="text-3xl font-bold text-gray-900 dark:text-white mb-4">
>>>>>>> b932655445289cc6885ffad4b922c05b464845b2
                Welcome to TeachLink
            </Text>
            <Text className="text-gray-600 dark:text-gray-300 mb-8 px-4 text-center">
                Share and consume knowledge on the go
            </Text>

            <TouchableOpacity
<<<<<<< HEAD
                className="bg-blue-600 px-6 py-3 rounded-lg w-full max-w-xs items-center mb-3"
                onPress={() =>
                    navigation.navigate('CourseViewer', {
                        course: sampleCourse,
                    })
                }
            >
                <Text className="text-white font-semibold">📚 Open Mobile Course Viewer</Text>
            </TouchableOpacity>

            <TouchableOpacity
                className="bg-blue-600 px-6 py-3 rounded-lg w-full max-w-xs items-center"
=======
                className="bg-blue-600 px-6 py-3 rounded-lg"
>>>>>>> b932655445289cc6885ffad4b922c05b464845b2
                onPress={() => navigation.navigate('Profile', { userId: '123' })}
            >
                <Text className="text-white font-semibold">Go to Profile</Text>
            </TouchableOpacity>

            <TouchableOpacity
<<<<<<< HEAD
                className="bg-gray-200 dark:bg-gray-700 px-6 py-3 rounded-lg mt-4 w-full max-w-xs items-center"
=======
                className="bg-gray-200 dark:bg-gray-700 px-6 py-3 rounded-lg mt-4"
>>>>>>> b932655445289cc6885ffad4b922c05b464845b2
                onPress={() => navigation.navigate('Settings')}
            >
                <Text className="text-gray-900 dark:text-white font-semibold">Settings</Text>
            </TouchableOpacity>
<<<<<<< HEAD
        </ScrollView>
=======
        </View>
>>>>>>> b932655445289cc6885ffad4b922c05b464845b2
    );
}