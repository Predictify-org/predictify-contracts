import React from 'react';
import { createDrawerNavigator } from '@react-navigation/drawer';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';
import { MobileTabBar } from './MobileTabBar';
import { MobileDrawer } from './MobileDrawer';
import { MobileHeader } from './MobileHeader';
import { View, Text } from 'react-native';

const Drawer = createDrawerNavigator();
const Tab = createBottomTabNavigator();

// Dummy Screens
const Screen = ({ title }: { title: string }) => (
    <View className="flex-1 bg-white items-center justify-center">
        <Text className="text-xl font-bold text-gray-800">{title}</Text>
        <Text className="text-gray-500 mt-2">Content goes here...</Text>
    </View>
);

const HomeScreen = () => <Screen title="Home Feed" />;
const ExploreScreen = () => <Screen title="Explore" />;
const CreateScreen = () => <Screen title="Create New" />;
const MessagesScreen = () => <Screen title="Messages" />;
const ProfileScreen = () => <Screen title="Profile" />;

function TabNavigator() {
    return (
        <Tab.Navigator
            tabBar={(props) => <MobileTabBar {...props} />}
            screenOptions={{
                headerShown: true,
                header: ({ route, options }) => (
                    <MobileHeader
                        title={options.title || route.name}
                        showBack={false}
                    />
                ),
            }}
        >
            <Tab.Screen name="Home" component={HomeScreen} />
            <Tab.Screen name="Explore" component={ExploreScreen} />
            <Tab.Screen name="Create" component={CreateScreen} options={{ tabBarLabel: () => null }} />
            <Tab.Screen name="Messages" component={MessagesScreen} />
            <Tab.Screen name="Profile" component={ProfileScreen} />
        </Tab.Navigator>
    );
}

export const SwipeableNavigation = () => {
    return (
        <Drawer.Navigator
            drawerContent={(props) => <MobileDrawer {...props} />}
            screenOptions={{
                headerShown: false,
                drawerType: 'slide',
                swipeEdgeWidth: 100, // Easier to swipe open
                drawerStyle: { width: '80%' },
            }}
        >
            <Drawer.Screen name="MainTabs" component={TabNavigator} />
        </Drawer.Navigator>
    );
};
