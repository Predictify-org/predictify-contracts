import React from 'react';
import { NavigationContainer } from '@react-navigation/native';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import { RootStackParamList } from './types';

// Import screens
import HomeScreen from '../screens/HomeScreen';
import ProfileScreen from '../screens/ProfileScreen';
import SettingsScreen from '../screens/SettingsScreen';
<<<<<<< HEAD
import CourseViewerScreen from '../screens/CourseViewerScreen';
=======
>>>>>>> b932655445289cc6885ffad4b922c05b464845b2

const Stack = createNativeStackNavigator<RootStackParamList>();

export default function AppNavigator() {
    return (
        <NavigationContainer>
            <Stack.Navigator initialRouteName="Home">
                <Stack.Screen
                    name="Home"
                    component={HomeScreen}
                    options={{ title: 'TeachLink' }}
                />
                <Stack.Screen name="Profile" component={ProfileScreen} />
                <Stack.Screen name="Settings" component={SettingsScreen} />
<<<<<<< HEAD
                <Stack.Screen
                    name="CourseViewer"
                    component={CourseViewerScreen}
                    options={{ title: 'Course', headerShown: false }}
                />
=======
>>>>>>> b932655445289cc6885ffad4b922c05b464845b2
            </Stack.Navigator>
        </NavigationContainer>
    );
}