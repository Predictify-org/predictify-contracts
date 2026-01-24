import React from 'react';
import { View, Text, Image, StyleSheet, ScrollView } from 'react-native';

interface Achievement {
  id: string;
  name: string;
  iconUrl?: string; // Optional URL for an icon
}

interface AchievementBadgesProps {
  achievements: Achievement[];
}

export const AchievementBadges: React.FC<AchievementBadgesProps> = ({ achievements }) => {
  return (
    <View style={styles.container}>
      <Text style={styles.title}>Achievements</Text>
      {achievements && achievements.length > 0 ? (
        <ScrollView horizontal showsHorizontalScrollIndicator={false} contentContainerStyle={styles.scrollContentContainer}>
          {achievements.map((achievement) => (
            <View key={achievement.id} style={styles.badgeContainer}>
              {achievement.iconUrl ? (
                <Image source={{ uri: achievement.iconUrl }} style={styles.badgeIcon} />
              ) : (
                // Placeholder icon if no URL is provided
                <View style={styles.badgePlaceholderIcon}>
                  <Text style={styles.badgePlaceholderText}>🎉</Text>
                </View>
              )}
              <Text style={styles.badgeName}>{achievement.name}</Text>
            </View>
          ))}
        </ScrollView>
      ) : (
        <Text style={styles.noAchievementsText}>No achievements yet.</Text>
      )}
    </View>
  );
};

const styles = StyleSheet.create({
  container: {
    marginVertical: 20,
    paddingHorizontal: 15,
  },
  title: {
    fontSize: 20,
    fontWeight: 'bold',
    marginBottom: 10,
    color: '#333',
  },
  scrollContentContainer: {
    paddingVertical: 10,
  },
  badgeContainer: {
    alignItems: 'center',
    marginRight: 20,
    width: 80, // Fixed width for each badge item
  },
  badgeIcon: {
    width: 50,
    height: 50,
    borderRadius: 25,
    marginBottom: 5,
    backgroundColor: '#e0e0e0', // Placeholder background if image fails to load
  },
  badgePlaceholderIcon: {
    width: 50,
    height: 50,
    borderRadius: 25,
    backgroundColor: '#007AFF',
    justifyContent: 'center',
    alignItems: 'center',
    marginBottom: 5,
  },
  badgePlaceholderText: {
    fontSize: 24,
  },
  badgeName: {
    fontSize: 12,
    textAlign: 'center',
    color: '#555',
  },
  noAchievementsText: {
    fontSize: 16,
    color: '#777',
    textAlign: 'center',
  },
});
