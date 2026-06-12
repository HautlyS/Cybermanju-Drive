<template>
  <div class="bw-flags">
    <div class="flag-line">
      <div v-for="i in 20" :key="i" class="flag-seg" :class="i % 2 === 0 ? 'white' : 'black'" />
    </div>
  </div>
</template>

<style scoped>
.bw-flags {
  position: relative;
  width: 100%;
  height: 4px;
  overflow: hidden;
  user-select: none;
  pointer-events: none;
  z-index: 1;
}

.flag-line {
  display: flex;
  height: 100%;
}

.flag-seg {
  flex: 1;
  height: 100%;
}

.flag-seg.white {
  background: #FFFFFF;
}

.flag-seg.black {
  background: #000000;
  border-right: 1px solid #FFFFFF;
}
</style>
