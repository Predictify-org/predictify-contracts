import React from 'react';
import { View, Text, TouchableOpacity } from 'react-native';
import { BottomTabBarProps } from '@react-navigation/bottom-tabs';
import { useSafeArea } from '../../hooks/useSafeArea';
import { Home, Compass, PlusCircle, MessageCircle, User } from 'lucide-react-native';

export const MobileTabBar = ({ state, descriptors, navigation }: BottomTabBarProps) => {
    const { bottom } = useSafeArea();

    const icons: Record<string, React.ReactNode> = {
        Home: <Home size={24} color="currentColor" />,
        Explore: <Compass size={24} color="currentColor" />,
        Create: <PlusCircle size={32} color="currentColor" />,
        Messages: <MessageCircle size={24} color="currentColor" />,
        Profile: <User size={24} color="currentColor" />,
    };

    return (
        <View
            className="flex-row bg-white border-t border-gray-200 shadow-sm"
            style={{ paddingBottom: bottom + 4 }}
        >
            {state.routes.map((route, index) => {
                const { options } = descriptors[route.key];
                const label = options.tabBarLabel !== undefined
                    ? options.tabBarLabel
                    : options.title !== undefined
                        ? options.title
                        : route.name;

                const isFocused = state.index === index;

                const onPress = () => {
                    const event = navigation.emit({
                        type: 'tabPress',
                        target: route.key,
                        canPreventDefault: true,
                    });

                    if (!isFocused && !event.defaultPrevented) {
                        navigation.navigate(route.name);
                    }
                };

                const onLongPress = () => {
                    navigation.emit({
                        type: 'tabLongPress',
                        target: route.key,
                    });
                };

                // Special styling for Create (middle button)
                if (route.name === 'Create') {
                    return (
                        <TouchableOpacity
                            key={index}
                            accessibilityRole="button"
                            accessibilityState={isFocused ? { selected: true } : {}}
                            accessibilityLabel={options.tabBarAccessibilityLabel}
                            onPress={onPress}
                            onLongPress={onLongPress}
                            className="flex-1 items-center justify-center -mt-6"
                        >
                            <View className="bg-indigo-600 rounded-full p-3 shadow-lg">
                                <PlusCircle size={32} color="white" />
                            </View>
                        </TouchableOpacity>
                    )
                }

                return (
                    <TouchableOpacity
                        key={index}
                        accessibilityRole="button"
                        accessibilityState={isFocused ? { selected: true } : {}}
                        accessibilityLabel={options.tabBarAccessibilityLabel}
                        onPress={onPress}
                        onLongPress={onLongPress}
                        className="flex-1 items-center justify-center pt-3 pb-1"
                    >
                        <View className={isFocused ? "text-indigo-600" : "text-gray-500"}>
                            {React.cloneElement(icons[route.name] as any, {
                                color: isFocused ? '#4F46E5' : '#6B7280'
                            })}
                        </View>
                        <Text
                            className={`text-xs mt-1 ${isFocused ? 'text-indigo-600 font-medium' : 'text-gray-500'}`}
                        >
                            {label.toString()}
                        </Text>
                    </TouchableOpacity>
                );
            })}
        </View>
    );
};
