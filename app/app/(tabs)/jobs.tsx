import { useRouter } from 'expo-router';
import { useState } from 'react';
import { FlatList, RefreshControl, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { api } from '../../src/api';
import { JobCard } from '../../src/components/JobCard';
import { Button, Empty, ErrorBanner, Header, IconButton, Loading, Segmented } from '../../src/components/ui';
import { space } from '../../src/theme';
import { useLoad } from '../../src/useLoad';

type Tab = 'new' | 'saved' | 'skipped';

export default function Jobs() {
  const router = useRouter();
  const [tab, setTab] = useState<Tab>('new');
  const { data, error, loading, refreshing, reload } = useLoad(() => api.jobs(tab), [tab]);

  return (
    <SafeAreaView style={{ flex: 1 }} edges={['top']}>
      <View style={{ paddingHorizontal: space(6) }}>
        <Header title="Jobs" right={<IconButton icon="plus" label="Add a job" onPress={() => router.push('/add-job')} />} />
        <Segmented
          value={tab}
          onChange={setTab}
          options={[
            { value: 'new', label: 'New' },
            { value: 'saved', label: 'Saved' },
            { value: 'skipped', label: 'Skipped' },
          ]}
        />
      </View>
      {error && <View style={{ padding: space(6) }}><ErrorBanner message={error} onRetry={reload} /></View>}
      {loading && !data ? (
        <Loading />
      ) : (
        <FlatList
          data={data ?? []}
          keyExtractor={(j) => j.id}
          contentContainerStyle={{ padding: space(6) }}
          refreshControl={<RefreshControl refreshing={refreshing} onRefresh={reload} />}
          renderItem={({ item }) => <JobCard job={item} onPress={() => router.push(`/job/${item.id}`)} />}
          ListEmptyComponent={
            <Empty
              icon={tab === 'new' ? 'inbox' : tab === 'saved' ? 'bookmark' : 'x-circle'}
              title={tab === 'new' ? 'No new matches' : tab === 'saved' ? 'Nothing saved' : 'Nothing skipped'}
              body={tab === 'new' ? 'New matches land here after each morning run.' : 'Jobs you save or skip from the queue show up here.'}
              action={tab === 'new' ? <Button label="Add a job you found" variant="soft" onPress={() => router.push('/add-job')} /> : undefined}
            />
          }
        />
      )}
    </SafeAreaView>
  );
}
