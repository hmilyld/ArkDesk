<!--
  加解密工具页：单页多标签，聚合哈希/编码/对称/非对称/派生/杂项。
-->
<script setup lang="ts">
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import ToolShell from '@/components/tool/ToolShell.vue';
import HashPanel from '../components/crypto/HashPanel.vue';
import EncodingPanel from '../components/crypto/EncodingPanel.vue';
import SymmetricPanel from '../components/crypto/SymmetricPanel.vue';
import AsymmetricPanel from '../components/crypto/AsymmetricPanel.vue';
import KdfPanel from '../components/crypto/KdfPanel.vue';
import MiscPanel from '../components/crypto/MiscPanel.vue';
</script>

<template>
  <ToolShell title="加解密" description="哈希、编码、对称/非对称与国密加解密">
    <Tabs default-value="hash" class="w-full">
      <TabsList class="mx-auto mb-2 w-fit">
        <TabsTrigger value="hash">摘要</TabsTrigger>
        <TabsTrigger value="encoding">编码</TabsTrigger>
        <TabsTrigger value="symmetric">对称</TabsTrigger>
        <TabsTrigger value="asymmetric">非对称</TabsTrigger>
        <TabsTrigger value="kdf">口令派生</TabsTrigger>
        <TabsTrigger value="misc">杂项</TabsTrigger>
      </TabsList>

      <TabsContent value="hash">
        <HashPanel />
      </TabsContent>
      <TabsContent value="encoding">
        <EncodingPanel />
      </TabsContent>
      <TabsContent value="symmetric">
        <SymmetricPanel />
      </TabsContent>
      <TabsContent value="asymmetric">
        <AsymmetricPanel />
      </TabsContent>
      <TabsContent value="kdf">
        <KdfPanel />
      </TabsContent>
      <TabsContent value="misc">
        <MiscPanel />
      </TabsContent>
    </Tabs>
  </ToolShell>
</template>
